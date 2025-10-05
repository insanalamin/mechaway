# mechaway
> **Hyperminimalist intelligent systems automation for open-source lovers.**

https://way.mecha.id

---

## 🧩 What is mechaway?

**mechaway** is a **workflow automation and data orchestration engine for intelligent systems**, written in **Rust**.  
It lets you visually connect nodes to build real-time, event-driven, and data-centric systems — from webhooks and APIs to IoT and AI.

Inspired by the core functionality of **Node-RED**, the friendly UX of **N8n**, and the minimalist of **Huggingface** formats:  
- predictable,  
- hot-reloadable,  
- zero-GC latency,  
- and open to anyone who believes simplicity can be powerful.

---

## 🚀 Some Features

| Principle | Description |
|------------|-------------|
| 🦀 **Hyperminimalist Core** | Built in Rust — no Node.js, no dependency bloat, no nonsense. |
| ⚙️ **Composable DAG Engine** | Each workflow is a Directed Acyclic Graph (DAG) executed in-memory using Petgraph for deterministic flow. |
| 🔥 **Hot-Reload Architecture** | Edit and save workflows — instantly live, no server restart. |
| 🌍 **Universal Connectivity** | First-class nodes for HTTP, WebSocket, gRPC, Pub/Sub, ArangoDB, TimescaleDB, and AI pipelines. |
| 🧠 **Intelligent Data Layer** | JSON and Tabular transformation nodes, powered by `jsonpath_lib` and `polars`. |
| 🧩 **Extensible by Design** | Add new nodes as `.so` or `.wasm` plugins — no pod restart, no rebuilds. |
| 💾 **Built-in Data Store** | Integrated lightweight data layer with simple tabular + vector DB and a minimalist network file system. |
| 🧠 **AI-Ready** | Integrate Hugging Face models, Piper TTS, and other open AI tools directly as node containers. |

---

- **Hot-reloadable DAGs** — stored in SQLite, executed fully in memory.  
- **Dynamic Plugin System** — supports `.so` (native) and `.wasm` (sandboxed) node packs.  
- **In-Process Execution** — ultra-fast native execution of logic and transformation nodes.  
- **Optional Distributed Mode** — scale across pods via gRPC or PubSub.
- **Built-in Storage** — tabular store, vector search, and networked file system for data persistence and caching.

---

## 🧩 Core Node Categories

| Type | Description |
|------|--------------|
| **Simple Data Nodes** | Lightweight storage and vector database nodes, including SQLite, simple key-value, and embedded vector stores. |
| **Simple Network Filesystem Node** | Minimalist distributed filesystem connector for file sharing and caching over the network. |
| **Data Nodes** | JSONTransform, TabularTransform, CSV/Parquet processing. |
| **Compute Nodes** | WASMNode, FunctionNode, DockerNode for local or external computation. |
| **Integration Nodes** | HTTP, WebSocket, PubSub, gRPC for inter-service communication. |
| **Database Nodes** | Postgres, ArangoDB, TimescaleDB for structured or graph-based data. |
| **AI / ML Nodes** | HuggingFace, Whisper, Piper, Llama for open-source AI inference. |
| **Trigger Nodes** | Webhook, Scheduler, EventStream for reactive and time-based triggers. |

---

## 💡 Vision

> To empower developers, researchers, and makers  
> with a **minimalist**, **reliable**, and **intelligent** automation platform  
> that honors the open-source ecosystem —  
> **fast enough for robotics, simple enough for data flows, and open enough for ideas.**

---

## 🛠️ Installation (Coming Soon)

```bash
cargo install mechaway
mechaway start
