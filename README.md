# Rust Agentic RPA

A powerful, modular AI Browser Agent built with Rust. This project enables an LLM to "see" the web, "think" about the next steps, and "interact" with elements to automate complex browser-based tasks.

## Overview

`rust-agentic-rpa` is a state-of-the-art Robotic Process Automation (RPA) tool that leverages Large Language Models (LLMs) to navigate the web autonomously. Unlike traditional RPA which relies on static selectors, this agent understands the DOM and can adapt to UI changes dynamically.

### How it Works

The agent follows an "Observe-Think-Act" cycle:

1. **Observe**: The agent captures a snapshot of the current web page, including the URL, page title, and a filtered DOM tree (to keep the context window efficient).
2. **Think**: The captured state is sent to the LLM (the Brain). The Brain analyzes the visual and structural data against the user's goal to decide the next logical step.
3. **Act**: The decided action (e.g., clicking a button, typing text, or navigating to a new URL) is executed by the browser controller (the Hands).

## Features

### Intelligent Decision Making (The Brain)

- **Context-Aware**: Processes DOM structure to locate interactive elements without hardcoded IDs or XPaths.
- **Error Recovery**: If an action fails or the page doesn't load as expected, the Brain can observe the error and attempt an alternative path.
- **Goal Completion**: Recognizes when a task is finished and provides a summary of the actions taken.

### Browser Orchestration (The Hands)

- **Headless Chrome Management**: Leverages `headless_chrome` for low-level control over browser sessions.
- **Human-like Interactions**: Simulates realistic mouse movements, clicks, and keystrokes.
- **Multi-Tab Support**: Can manage multiple tabs simultaneously for complex workflows.
- **Smart Waiting**: Automatically waits for elements to appear or for the page to reach a "body-ready" state before proceeding.

### Communication Interface (The Face)

- **Event Streaming**: Provides a broadcast system to stream real-time events (thinking, steps, errors) to external consumers.
- **Web Dashboard**: An internal Axum-based server that acts as a bridge between the agent core and the user interface.

### Security and Persistence

- **Secure Profiles**: Uses isolated "Shadow Profiles" for Chrome to maintain cookies and sessions without interfering with your main browser.
- **Environment Safety**: Sensitive configurations like API keys are managed via `.env` files and are automatically ignored by Git.

## Architecture Deep Dive

The project is architected as a set of autonomous but interconnected modules:

### 🧠 Brain (`src/bin/agent/brain.rs`)

The cognitive center. It builds a prompt for the LLM that includes the user's ultimate goal and the history of observations. It interprets the LLM's raw text response into structured `Step` commands.

### 👐 Hands (`src/bin/agent/hands.rs`)

The physical interface. It handles the `headless_chrome::Browser` and `Tab` instances. It translates high-level commands like `TypeInto` or `Click` into DevTools Protocol requests.

### 👁️ Face (`src/bin/agent/face.rs`)

The communication layer. It uses Axum to host a server that broadcasts `AgentEvent` updates. This allows for real-time monitoring of the agent's circular reasoning and activities.

### 🌳 DOM (`src/bin/agent/dom.rs`)

The sensory module. It contains utilities to capture the current DOM state, calculate viewport positions, and extract text/content from specific selectors.

## Getting Started

### Prerequisites

- **Rust**: Latest stable version (Edition 2024).
- **Google Chrome**: Installed on your system.
- **OpenAI API Key**: Required for the agent's "Brain".

### Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/Fernandoleano/Rust-Agentic-RPA.git
   cd Rust-Agentic-RPA
   ```

2. Setup your environment:
   Create a `.env` file in the root directory:

   ```env
   OPENAI_API_KEY=your_actual_key_here
   ```

3. Build the project:
   ```bash
   cargo build
   ```

## Running the Agent

Start the main orchestrator:

```bash
cargo run --bin agent
```

Once started, the agent will:

1. Launch the communication server (Face).
2. Start an isolated Chrome session.
3. Wait for commands. You can interact with the agent through the system-generated UI or by sending commands to the broadcast bridge.

## Project Structure

```text
src/
└── bin/
    └── agent/
        ├── main.rs   # Entry point: Orchestrates Brain, Hands, and Face.
        ├── brain.rs  # Cognitive Module: Decision making and intent parsing.
        ├── hands.rs  # Physical Module: Browser session and tab control.
        ├── face.rs   # Interface Module: Web server and event broadcasting.
        ├── dom.rs    # Sensory Module: DOM traversal and snapshot capture.
        └── types.rs  # Shared Types: Definitions for Steps, Events, and State.
```

## Technical Details

- **Runtime**: Powered by `tokio` (multi-threaded).
- **Error Handling**: Uses `anyhow` for robust and descriptive error propagation.
- **Serialization**: `serde` and `serde_json` for efficient data exchange between the Brain and the Browser.

## License

MIT License. See [LICENSE](LICENSE) for details.
