use anyhow::{Result, anyhow};
use reqwest::Client;
use serde_json::json;

use crate::types::{ChatMessage, PageState, Step};

const MODEL: &str = "gpt-5.2"; // Change to "gpt-5.2" or your preferred model

const SYSTEM_PROMPT: &str = r#"You are a browser automation agent. You control a real Chrome browser by issuing ONE step at a time as JSON.

Available actions:
- {"action":"Navigate","url":"https://..."}
- {"action":"WaitFor","selector":"[data-eid=\"[e0]\"]","timeout_ms":5000}
- {"action":"TypeInto","selector":"[data-eid=\"[e0]\"]","text":"search query"}
- {"action":"Click","selector":"[data-eid=\"[e0]\"]"}
- {"action":"PressKey","key":"Enter"}
- {"action":"Extract","selector":"body","label":"main_content"}
- {"action":"Screenshot"}
- {"action":"NewTab"}
- {"action":"Done","summary":"Completed: found the answer is 42"}

Rules:
1. Return ONLY a single JSON object per response. No markdown, no explanation.
2. Use the [eN] element IDs from the DOM snapshot to target elements. Use selector format: [data-eid="[eN]"]
3. After Navigate, the system will show you the new page DOM. Decide your next step based on what you see.
4. Use TypeInto to fill inputs, then PressKey with "Enter" to submit. Or Click the submit button.
5. When the user's task is accomplished, use Done with a summary of what was achieved.
6. If you encounter an error, try an alternative approach. If stuck after 3 attempts, use Done to explain.
7. Keep steps minimal. Do not over-navigate."#;

pub struct Brain {
    client: Client,
    api_key: String,
    conversation: Vec<ChatMessage>,
    memory_path: std::path::PathBuf,
}

impl Brain {
    pub fn new() -> Result<Self> {
        let api_key = std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow!("OPENAI_API_KEY not set in environment"))?;

        let conversation = vec![ChatMessage {
            role: "system".to_string(),
            content: SYSTEM_PROMPT.to_string(),
        }];

        let mut brain = Self {
            client: Client::new(),
            api_key,
            conversation,
            memory_path: std::path::PathBuf::from("memory.json"),
        };

        // Try to load existing memory
        brain.load_memory();

        Ok(brain)
    }

    fn load_memory(&mut self) {
        if let Ok(file) = std::fs::File::open(&self.memory_path) {
            let reader = std::io::BufReader::new(file);
            if let Ok(saved_msgs) = serde_json::from_reader::<_, Vec<ChatMessage>>(reader) {
                // Validate system prompt matches
                if !saved_msgs.is_empty() && saved_msgs[0].role == "system" {
                    eprintln!("[Brain] Loaded {} messages from memory.", saved_msgs.len());
                    self.conversation = saved_msgs;
                }
            }
        }
    }

    fn save_memory(&self) {
        if let Ok(file) = std::fs::File::create(&self.memory_path) {
            let writer = std::io::BufWriter::new(file);
            let _ = serde_json::to_writer_pretty(writer, &self.conversation);
        }
    }

    /// Start a new task. Preserves history/context.
    pub fn start_task(&mut self, user_prompt: &str) {
        // self.conversation.truncate(1); // OLD: Wiped history

        // NEW: Append to history
        self.conversation.push(ChatMessage {
            role: "user".to_string(),
            content: format!(
                "Task: {}\n\nThe browser is on the current page. What is your next step?",
                user_prompt
            ),
        });
        self.save_memory();
    }

    /// Feed observation back to the LLM.
    pub fn observe(&mut self, page_state: &PageState) {
        let mut observation = format!(
            "Page URL: {}\nTitle: {}\n\nDOM:\n{}",
            page_state.url, page_state.title, page_state.dom_snapshot
        );

        if let Some(ref err) = page_state.error {
            observation.push_str(&format!("\n\nERROR from last step: {}", err));
        }

        for ext in &page_state.extracted {
            observation.push_str(&format!("\n\nExtracted [{}]: {}", ext.label, ext.content));
        }

        self.conversation.push(ChatMessage {
            role: "user".to_string(),
            content: observation,
        });
        self.save_memory();
    }

    /// Ask the LLM for the next step.
    pub async fn decide_next_step(&mut self) -> Result<Step> {
        let messages: Vec<serde_json::Value> = self
            .conversation
            .iter()
            .map(|m| json!({"role": m.role, "content": m.content}))
            .collect();

        // Check token limit helper (naive)
        if messages.len() > 20 {
            eprintln!(
                "[Brain] Warning: Conversation history is long ({})",
                messages.len()
            );
        }

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&json!({
                "model": MODEL,
                "messages": messages,
                "temperature": 0.2,
            }))
            .send()
            .await?;

        let status = response.status();
        let json_resp: serde_json::Value = response.json().await?;

        if !status.is_success() {
            let err_msg = json_resp["error"]["message"]
                .as_str()
                .unwrap_or("Unknown API error");
            eprintln!("[Brain] API error ({}): {}", status, err_msg);
            return Err(anyhow!("OpenAI API error ({}): {}", status, err_msg));
        }

        let content = json_resp["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                eprintln!("[Brain] Unexpected response: {}", json_resp);
                anyhow!("No content in LLM response: {}", json_resp)
            })?;

        eprintln!("[Brain] LLM says: {}", content);

        // Record assistant response in conversation history
        self.conversation.push(ChatMessage {
            role: "assistant".to_string(),
            content: content.to_string(),
        });
        self.save_memory(); // Save after assistant reply

        // Strip possible markdown fences the LLM might add
        let cleaned = content
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        eprintln!("[Brain] Cleaned JSON: {}", cleaned);

        let step: Step = match serde_json::from_str(cleaned) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Brain] JSON Parse Error: {}. Content: {}", e, cleaned);
                return Err(anyhow!("Failed to parse LLM response: {}", e));
            }
        };

        Ok(step)
    }
}
