use serde::{Deserialize, Serialize};

/// A single atomic step the LLM asks the agent to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Step {
    Navigate { url: String },
    WaitFor { selector: String, timeout_ms: u64 },
    TypeInto { selector: String, text: String },
    Click { selector: String },
    PressKey { key: String },
    Extract { selector: String, label: String },
    Screenshot,
    Done { summary: String },
    NewTab,
}

/// What the agent observes after executing a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageState {
    pub url: String,
    pub title: String,
    pub dom_snapshot: String,
    pub extracted: Vec<Extraction>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extraction {
    pub label: String,
    pub content: String,
}

/// A message in the conversation history sent to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

pub const MAX_STEPS_PER_TASK: usize = 25;
pub const DOM_SNAPSHOT_MAX_CHARS: usize = 4000;
