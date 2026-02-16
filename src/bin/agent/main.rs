mod brain;
mod dom;
mod face;
mod hands;
mod types;

use anyhow::Result;
use dotenvy::dotenv;
use face::AgentEvent;
use tokio::sync::broadcast;
use types::{MAX_STEPS_PER_TASK, Step};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    eprintln!("[Agent] Starting AI Browser Agent...");

    // 1. Launch web UI first (so user sees something immediately)
    let (mut cmd_rx, event_tx) = face::start_server().await;

    // 2. Launch browser in a blocking task (it can take a while)
    eprintln!("[Agent] Launching Chrome...");
    let mut session = tokio::task::spawn_blocking(|| hands::BrowserSession::launch())
        .await
        .map_err(|e| anyhow::anyhow!("Browser launch panicked: {}", e))??;
    eprintln!("[Agent] Chrome launched successfully.");

    let mut brain = brain::Brain::new()?;
    eprintln!("[Agent] Brain ready. Waiting for commands...");

    // 3. Wait for commands from the web UI
    while let Some(user_command) = cmd_rx.recv().await {
        eprintln!("[Agent] Received command: '{}'", user_command);
        run_task(&mut session, &mut brain, &user_command, &event_tx).await;
    }

    Ok(())
}

async fn run_task(
    session: &mut hands::BrowserSession,
    brain: &mut brain::Brain,
    command: &str,
    events: &broadcast::Sender<AgentEvent>,
) {
    brain.start_task(command);

    // Always start a new task in a new tab
    if let Err(e) = session.new_tab() {
        eprintln!("[Agent] Warning: Failed to open new tab for task: {}", e);
    }

    let mut step_count = 0;

    loop {
        if step_count >= MAX_STEPS_PER_TASK {
            eprintln!("[Agent] Step limit reached");
            let _ = events.send(AgentEvent::TaskError {
                message: format!("Reached maximum step limit ({})", MAX_STEPS_PER_TASK),
            });
            break;
        }

        eprintln!("[Agent] Asking Brain (LLM) for next step...");
        let _ = events.send(AgentEvent::Thinking);

        let step_result = brain.decide_next_step().await;
        eprintln!("[Agent] Brain replied. Result: {:?}", step_result);

        let step = match step_result {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Agent] LLM error: {:#}", e);
                let _ = events.send(AgentEvent::TaskError {
                    message: format!("{:#}", e),
                });
                break;
            }
        };

        step_count += 1;

        if let Step::Done { ref summary } = step {
            eprintln!("[Agent] Task complete: {}", summary);
            let _ = events.send(AgentEvent::TaskComplete {
                summary: summary.clone(),
            });
            break;
        }

        // Handle NewTab specially (requires session, not just tab)
        if let Step::NewTab = step {
            eprintln!("[Agent] Opening new tab...");
            if let Err(e) = session.new_tab() {
                eprintln!("[Agent] Failed to open new tab: {}", e);
            }
        }

        let description = format!("{:?}", step);
        eprintln!("[Agent] Step {}: {}", step_count, description);
        let _ = events.send(AgentEvent::Step {
            number: step_count,
            description,
        });

        // Execute in a blocking context so we don't stall tokio
        let tab = session.tab.clone();
        let step_clone = step.clone();
        let page_state = tokio::task::spawn_blocking(move || {
            let mut extracted = Vec::new();
            let mut error = None;

            match execute_step_on_tab(&tab, &step_clone, &mut extracted) {
                Ok(()) => {}
                Err(e) => error = Some(format!("{:#}", e)),
            }

            let url = crate::dom::get_current_url(&tab).unwrap_or_else(|_| "unknown".into());
            let title = crate::dom::get_page_title(&tab).unwrap_or_else(|_| "untitled".into());
            let dom_snapshot =
                crate::dom::capture_dom_snapshot(&tab).unwrap_or_else(|_| String::new());

            types::PageState {
                url,
                title,
                dom_snapshot,
                extracted,
                error,
            }
        })
        .await
        .unwrap();

        if let Some(ref err) = page_state.error {
            eprintln!("[Agent] Step error: {}", err);
            let _ = events.send(AgentEvent::StepError {
                message: err.clone(),
            });
        }

        brain.observe(&page_state);
    }

    let _ = events.send(AgentEvent::Ready);
}

/// Execute a step using just the Arc<Tab> (so it can run in spawn_blocking).
fn execute_step_on_tab(
    tab: &std::sync::Arc<headless_chrome::Tab>,
    step: &Step,
    extracted: &mut Vec<types::Extraction>,
) -> Result<()> {
    use std::time::Duration;

    match step {
        Step::Navigate { url } => {
            tab.navigate_to(url)?;
            tab.wait_for_element("body")?;
            std::thread::sleep(Duration::from_millis(1500));
        }
        Step::WaitFor {
            selector,
            timeout_ms,
        } => {
            tab.wait_for_element_with_custom_timeout(selector, Duration::from_millis(*timeout_ms))?;
        }
        Step::TypeInto { selector, text } => {
            let el = tab.find_element(selector)?;
            el.click()?;
            let js_sel = selector.replace('\'', "\\'");
            tab.evaluate(
                &format!("document.querySelector('{js_sel}').value = ''"),
                false,
            )?;
            tab.type_str(text)?;
        }
        Step::Click { selector } => {
            let el = tab.find_element(selector)?;
            el.click()?;
            std::thread::sleep(Duration::from_millis(1000));
        }
        Step::PressKey { key } => {
            tab.press_key(key)?;
            std::thread::sleep(Duration::from_millis(1000));
        }
        Step::Extract { selector, label } => {
            let js_sel = selector.replace('\'', "\\'");
            let result = tab.evaluate(
                &format!("(document.querySelector('{js_sel}') || {{}}).innerText || ''"),
                false,
            )?;
            let content = result
                .value
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default();
            extracted.push(types::Extraction {
                label: label.clone(),
                content: content.chars().take(2000).collect(),
            });
        }
        Step::Screenshot | Step::Done { .. } | Step::NewTab => {}
    }

    Ok(())
}
