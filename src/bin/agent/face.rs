use axum::Json;
use axum::Router;
use axum::extract::State;
use axum::response::Html;
use axum::response::sse::{Event, Sse};
use axum::routing::{get, post};
use serde::Deserialize;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

/// Events streamed to the browser via SSE.
#[derive(Clone, Debug)]
pub enum AgentEvent {
    Step { number: usize, description: String },
    StepError { message: String },
    TaskComplete { summary: String },
    TaskError { message: String },
    Thinking,
    Ready,
}

impl AgentEvent {
    fn to_sse_event(&self) -> Event {
        match self {
            AgentEvent::Step {
                number,
                description,
            } => Event::default().event("step").data(format!(
                "{{\"number\":{},\"description\":{}}}",
                number,
                serde_json::json!(description)
            )),
            AgentEvent::StepError { message } => Event::default()
                .event("step_error")
                .data(format!("{{\"message\":{}}}", serde_json::json!(message))),
            AgentEvent::TaskComplete { summary } => Event::default()
                .event("task_complete")
                .data(format!("{{\"summary\":{}}}", serde_json::json!(summary))),
            AgentEvent::TaskError { message } => Event::default()
                .event("task_error")
                .data(format!("{{\"message\":{}}}", serde_json::json!(message))),
            AgentEvent::Thinking => Event::default().event("thinking").data("{}"),
            AgentEvent::Ready => Event::default().event("ready").data("{}"),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub cmd_tx: mpsc::Sender<String>,
    pub event_tx: broadcast::Sender<AgentEvent>,
}

#[derive(Deserialize)]
struct CommandPayload {
    command: String,
}

/// Start the web server on localhost:3000. Returns the shared channels.
pub async fn start_server() -> (mpsc::Receiver<String>, broadcast::Sender<AgentEvent>) {
    let (cmd_tx, cmd_rx) = mpsc::channel::<String>(1);
    let (event_tx, _) = broadcast::channel::<AgentEvent>(64);

    let state = Arc::new(AppState {
        cmd_tx,
        event_tx: event_tx.clone(),
    });

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/command", post(command_handler))
        .route("/events", get(sse_handler))
        .route(
            "/favicon.ico",
            get(|| async { axum::http::StatusCode::NO_CONTENT }),
        ) // Silence 404
        .with_state(state);

    // Try port 3000, fall back to 3001-3009 if in use
    let mut listener = None;
    let mut port = 3000;
    for p in 3000..3010 {
        match tokio::net::TcpListener::bind(format!("127.0.0.1:{}", p)).await {
            Ok(l) => {
                listener = Some(l);
                port = p;
                break;
            }
            Err(_) => continue,
        }
    }
    let listener =
        listener.expect("Could not bind to any port 3000-3009. Kill the old agent first.");

    eprintln!("Web UI running at http://localhost:{}", port);

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (cmd_rx, event_tx)
}

async fn index_handler() -> Html<&'static str> {
    eprintln!("[Web] GET /");
    Html(INDEX_HTML)
}

async fn command_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CommandPayload>,
) -> &'static str {
    eprintln!("[Web] POST /command: {}", payload.command);
    let _ = state.cmd_tx.send(payload.command).await;
    "ok"
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_tx.subscribe();
    let stream =
        BroadcastStream::new(rx).filter_map(|result: Result<AgentEvent, _>| match result {
            Ok(event) => Some(Ok::<_, Infallible>(event.to_sse_event())),
            Err(_) => None,
        });
    Sse::new(stream)
}

const INDEX_HTML: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>AI Browser Agent</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    background: #0a0a0f;
    color: #e0e0e0;
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }
  header {
    padding: 24px 32px;
    border-bottom: 1px solid #1a1a2e;
    display: flex;
    align-items: center;
    gap: 12px;
  }
  header h1 {
    font-size: 20px;
    font-weight: 600;
    color: #fff;
  }
  header .dot {
    width: 8px; height: 8px;
    border-radius: 50%;
    background: #22c55e;
    animation: pulse 2s infinite;
  }
  header .dot.busy { background: #f59e0b; }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
  .main {
    flex: 1;
    display: flex;
    flex-direction: column;
    max-width: 800px;
    width: 100%;
    margin: 0 auto;
    padding: 24px 32px;
    gap: 16px;
    overflow: hidden;
  }
  #log {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-right: 8px;
  }
  #log::-webkit-scrollbar { width: 6px; }
  #log::-webkit-scrollbar-track { background: transparent; }
  #log::-webkit-scrollbar-thumb { background: #333; border-radius: 3px; }
  .entry {
    padding: 10px 14px;
    border-radius: 8px;
    font-size: 14px;
    line-height: 1.5;
    animation: fadeIn 0.2s ease;
  }
  @keyframes fadeIn { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; } }
  .entry.user {
    background: #1a1a2e;
    border-left: 3px solid #6366f1;
  }
  .entry.step {
    background: #111118;
    border-left: 3px solid #3b82f6;
    font-family: 'Cascadia Code', 'Fira Code', monospace;
    font-size: 13px;
  }
  .entry.step .num {
    color: #6366f1;
    font-weight: 700;
    margin-right: 8px;
  }
  .entry.error {
    background: #1a0a0a;
    border-left: 3px solid #ef4444;
    color: #fca5a5;
  }
  .entry.done {
    background: #0a1a0a;
    border-left: 3px solid #22c55e;
    color: #86efac;
  }
  .entry.thinking {
    background: #111118;
    border-left: 3px solid #f59e0b;
    color: #fcd34d;
  }
  .input-area {
    display: flex;
    gap: 8px;
  }
  #cmd {
    flex: 1;
    background: #111118;
    border: 1px solid #222;
    border-radius: 8px;
    padding: 12px 16px;
    color: #fff;
    font-size: 16px;
    outline: none;
    transition: border-color 0.2s;
  }
  #cmd:focus { border-color: #6366f1; }
  #cmd::placeholder { color: #555; }
  #cmd:disabled { opacity: 0.5; }
  button {
    background: #6366f1;
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 12px 24px;
    font-size: 15px;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s;
  }
  button:hover { background: #4f46e5; }
  button:disabled { background: #333; cursor: not-allowed; }
</style>
</head>
<body>
  <header>
    <div class="dot" id="status-dot"></div>
    <h1>AI Browser Agent</h1>
  </header>
  <div class="main">
    <div id="log"></div>
    <div class="input-area">
      <input type="text" id="cmd" placeholder="Tell the agent what to do..." autofocus />
      <button id="send" onclick="send()">Send</button>
    </div>
  </div>
<script>
  const log = document.getElementById('log');
  const cmd = document.getElementById('cmd');
  const sendBtn = document.getElementById('send');
  const dot = document.getElementById('status-dot');
  let busy = false;

  function addEntry(cls, html) {
    const div = document.createElement('div');
    div.className = 'entry ' + cls;
    div.innerHTML = html;
    log.appendChild(div);
    log.scrollTop = log.scrollHeight;
  }

  function setBusy(b) {
    busy = b;
    cmd.disabled = b;
    sendBtn.disabled = b;
    dot.className = b ? 'dot busy' : 'dot';
    if (!b) cmd.focus();
  }

  async function send() {
    const text = cmd.value.trim();
    if (!text || busy) return;
    cmd.value = '';
    addEntry('user', '<strong>You:</strong> ' + text.replace(/</g,'&lt;'));
    setBusy(true);
    await fetch('/command', {
      method: 'POST',
      headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({command: text}),
    });
  }

  cmd.addEventListener('keydown', e => {
    if (e.key === 'Enter') send();
  });

  const es = new EventSource('/events');

  es.addEventListener('step', e => {
    const d = JSON.parse(e.data);
    addEntry('step', '<span class="num">Step ' + d.number + '</span>' + d.description.replace(/</g,'&lt;'));
  });

  es.addEventListener('step_error', e => {
    const d = JSON.parse(e.data);
    addEntry('error', '<strong>Error:</strong> ' + d.message.replace(/</g,'&lt;'));
  });

  es.addEventListener('task_complete', e => {
    const d = JSON.parse(e.data);
    addEntry('done', '<strong>Done:</strong> ' + d.summary.replace(/</g,'&lt;'));
    setBusy(false);
  });

  es.addEventListener('task_error', e => {
    const d = JSON.parse(e.data);
    addEntry('error', '<strong>Task failed:</strong> ' + d.message.replace(/</g,'&lt;'));
    setBusy(false);
  });

  es.addEventListener('thinking', () => {
    addEntry('thinking', 'Thinking...');
  });

  es.addEventListener('ready', () => {
    setBusy(false);
  });

  addEntry('done', 'Agent ready. Type a command to begin.');
</script>
</body>
</html>
"##;
