# Rust Agentic RPA

A powerful, modular AI Browser Agent built with Rust. This project enables an LLM to "see" the web, "think" about the next steps, and "interact" with elements to automate complex browser-based tasks.

## Overview

`rust-agentic-rpa` is a state-of-the-art Robotic Process Automation (RPA) tool that leverages Large Language Models (LLMs) to navigate the web autonomously. Unlike traditional RPA which relies on static selectors, this agent understands the DOM and can adapt to UI changes dynamically.

## Features

- **Intelligent Brain**: Driven by OpenAI (via `anyhow` and `tokio`), the agent processes DOM snapshots to decide the best course of action.
- **Hardware Interaction (Hands)**: Utilizes `headless_chrome` to perform human-like actions:
  - Navigation & Tab management.
  - Typing, Clicking, and Key Pressing.
  - Data Extraction.
- **Real-time Face**: A built-in web server (Axum) provides a communication layer and dashboard to monitor the agent's progress and events.
- **High Performance**: Built on the `tokio` asynchronous runtime for efficient browser control and event handling.
- **Secure Profiling**: Automatically manages Chrome "Shadow Profiles" to maintain persistent sessions safely.

## Architecture

The project is divided into logical modules representing the agent's anatomy:

| Module  | Description                                                               |
| :------ | :------------------------------------------------------------------------ |
| `Brain` | The LLM interface that handles decision-making and observation analysis.  |
| `Hands` | The browser control layer (based on `headless_chrome`).                   |
| `Face`  | The communication layer (Web Server/Events) that interacts with the user. |
| `DOM`   | Essential utilities for capturing snapshots and extracting page data.     |

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

1. Launch its Web UI (Face).
2. Start a persistent Chrome session.
3. Wait for your commands via the web interface.

## Project Structure

```text
src/
└── bin/
    └── agent/
        ├── main.rs   # Entry point & Orchestration
        ├── brain.rs  # LLM Decision Logic
        ├── hands.rs  # Browser Control (Chrome)
        ├── face.rs   # Web Server & UI Communication
        └── dom.rs    # Snapshot Utilities
```

## License

MIT License. See [LICENSE](LICENSE) for details.
