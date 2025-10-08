![Mechaway Logo](../uxui/logo-320.png)

# Mechaway Backend

> **Hyperminimalist intelligent systems automation engine** - Rust backend implementation

A modular, high-performance workflow automation engine built with Rust, featuring hot-reload capabilities, petgraph-based DAG execution, and extensible node system.

## 🏗️ Architecture

The backend follows a clean layered architecture with strict separation of concerns:

```
src/
├── main.rs              # 🚀 Minimal bootstrap
├── lib.rs               # 📦 Public API exports  
├── config/mod.rs        # ⚙️ Environment-aware configuration
├── project/             # 🏢 Multi-tenant Project System
│   ├── mod.rs           # Project module exports
│   ├── types.rs         # Project struct and helpers
│   └── database.rs      # ProjectDatabaseManager (isolated SQLite pools)
├── workflow/            # 🧠 Workflow Manager Layer
│   ├── types.rs         # Core workflow structs (Workflow, Node, Edge, ExecutionContext)
│   ├── storage.rs       # SQLite persistence with CRUD operations
│   └── registry.rs      # ArcSwap-based hot-reload registry
├── runtime/             # ⚡ Runtime Engine Layer  
│   ├── engine.rs        # Petgraph DAG execution engine
│   ├── executor.rs      # Individual node execution handlers + Safe Lua
│   └── scheduler.rs     # Industrial-grade hot-reload cron scheduler
├── api/                 # 🌐 HTTP API Layer
│   ├── workflows.rs     # Workflow CRUD endpoints with hot-reload integration
│   └── webhooks.rs      # Dynamic webhook execution routes
└── server.rs            # 🖥️ Axum server setup and project isolation wiring
```

## 🧩 Core Node Types

The system implements **7 production-ready node types** for comprehensive workflow automation:

### 🔗 WebhookNode
- **Purpose**: HTTP trigger entry points for workflow execution
- **Params**: `{ "path": "/create" }`
- **Behavior**: Creates dynamic webhook endpoints at `/webhook/{workflow_id}/{path}`

### 🧠 FunLogicNode  
- **Purpose**: **Safe sandboxed Lua** script execution for data transformation
- **Params**: `{ "script": "return {result = data.score * 2}" }`
- **Behavior**: Executes **whitelisted Lua** with restricted globals (no `os`, `io`, `debug`)

### ⏰ CronTriggerNode
- **Purpose**: **Hot-reload cron scheduling** with zero-downtime updates
- **Params**: `{ "schedule": "0 */3 * * * *", "timezone": "UTC" }`
- **Behavior**: **Industrial-grade scheduler** with job UUID tracking and proper cleanup

### 🌐 HTTPClientNode
- **Purpose**: External API calls and HTTP requests
- **Params**: `{ "url": "https://api.example.com", "method": "GET", "headers": {...} }`
- **Behavior**: Async HTTP client with response data forwarding

### 📊 SimpleTableWriterNode
- **Purpose**: **Project-isolated SQLite** data storage with auto-table creation
- **Params**: `{ "table": "blog_posts", "columns": ["title", "content", "author"] }`
- **Behavior**: **Lazy database creation** with project-scoped isolation

### 📖 SimpleTableReaderNode / SimpleTableQueryNode
- **Purpose**: **Project-scoped data retrieval** with SQL query support
- **Params**: `{ "table": "blog_posts", "where": "status = 'published'", "limit": 10 }`
- **Behavior**: **Safe SQL queries** within project database boundaries

### 🐘 PGQueryNode *(New)*
- **Purpose**: **PostgreSQL integration** with mandatory secret vault authentication
- **Params**: `{ "query": "SELECT * FROM users WHERE id = $1" }`
- **Secrets**: `["$secret.database_url"]` *(Required - no fallbacks)*
- **Behavior**: **Secure database access** with secret-based connection strings

## 🚀 Key Features

### 🏢 **Multi-Tenant Project System**
- **Project-isolated databases**: `data/{project_slug}/project.db` and `data/{project_slug}/simpletable.db`
- **Zero cross-project data leaks**: Complete database isolation per project
- **Lazy loading**: Database pools created only when accessed
- **Environment-aware**: `MECHAWAY_DATA_DIR` configuration support

### 🔥 **Industrial-Grade Hot-Reload**
- **Zero-downtime cron updates**: Update schedules without server restarts
- **Job UUID tracking**: Proper cleanup of old cron jobs to prevent duplicates
- **Workflow hot-reload**: Add/update/delete workflows with instant schedule sync
- **Scalable pattern**: Handles thousands of workflows with efficient resource management

### 🔒 **Safe Lua Execution**
- **Sandboxed environment**: Restricted access to dangerous globals (`os`, `io`, `debug`, `package`)
- **Whitelisted functions**: Only safe time functions (`date`, `time`, `now`) allowed
- **Input pin evaluation**: N8n-style `$json.field.path` and Lua expressions
- **Single-line expressions**: No multiline scripts for security

### 🔐 **Secret Vault System** *(Planned)*
- **Encrypted storage**: Project-scoped secret management
- **Mandatory authentication**: PGQuery nodes require secrets (no fallbacks)
- **N8n-style syntax**: `$secret.database_url` expressions
- **Database integration**: Stored in project-specific `project_secrets` table

## 🏗️ Technical Implementation

### Project Database Architecture
- **Project Database**: `{project_slug}/project.db` - workflows, secrets, metadata
- **SimpleTable Database**: `{project_slug}/simpletable.db` - node execution data
- **Complete isolation**: Zero cross-project data access
- **Lazy loading**: Connection pools created on-demand

### Lock-Free Hot Reload with Cron Integration
- Uses `ArcSwap` for atomic workflow registry updates
- **Hot-reload cron scheduler**: Updates schedules without server restarts
- **Job UUID tracking**: Prevents duplicate cron job accumulation
- Sub-millisecond reload performance with zero execution interruption

### Petgraph DAG Engine with Safe Execution
- Converts workflow JSON to directed acyclic graphs
- Topological sorting for execution order
- **Safe Lua sandboxing**: Restricted globals and whitelisted functions
- **Project-scoped execution**: All nodes operate within project boundaries

### Dynamic Webhook System
- Auto-generates HTTP routes based on workflow definitions
- Format: `/webhook/{workflow_id}/{webhook_path}`
- Runtime route registration and deregistration
- Flexible HTTP method support

## 📡 API Endpoints

### Workflow Management
```bash
# Create a new workflow
POST /api/workflows
Content-Type: application/json
Body: { "workflow": { ... } }

# List all workflows  
GET /api/workflows
Response: { "workflows": [...] }

# Get specific workflow
GET /api/workflows/{id}
Response: { "id": "...", "name": "...", "nodes": [...], "edges": [...] }

# Update workflow (triggers hot reload)
PUT /api/workflows/{id}
Body: { "workflow": { ... } }

# Delete workflow
DELETE /api/workflows/{id}
```

### Dynamic Execution
```bash
# Execute workflow via webhook trigger
POST /webhook/{workflow_id}/{webhook_path}
Content-Type: application/json
Body: { "student_id": "s123", "score": 85 }
```

### Health Check
```bash
# Server health probe
GET /healthz
Response: "ok"
```

## 🔧 Getting Started

### Prerequisites
- Rust 1.70+
- SQLite (automatically handled by sqlx)

### Build and Run
```bash
# Navigate to backend directory
cd axum-backend

# Install dependencies and build
cargo build --release

# Run the server (defaults to localhost:3004)
cargo run

# Or run in development mode with hot reloading
cargo run --bin mechaway
```

### Configuration
Default configuration (can be customized in `src/config/mod.rs`):
```rust
ServerConfig {
    host: "0.0.0.0",
    port: 3004,
}

DatabaseConfig {
    workflow_db_url: "sqlite:workflows.db",
    data_db_url: "sqlite:data.db", 
}
```

## 🧪 Testing the POC

### 1. Create Test Workflow
```bash
curl -X POST http://localhost:3004/api/workflows \
  -H "Content-Type: application/json" \
  -d @../test-workflow.json
```

### 2. Execute Workflow
```bash
curl -X POST http://localhost:3004/webhook/poc-grading-workflow/grade \
  -H "Content-Type: application/json" \
  -d '{
    "student_id": "s123", 
    "score": 85
  }'
```

### Expected Flow
1. **WebhookNode**: Receives HTTP request with student data
2. **FunLogicNode**: Processes score (doubles it, checks if >= 70 for pass/fail)
3. **SimpleTableWriterNode**: Stores result in `student_grades` table

### Sample Response
```json
{
  "student_id": "s123",
  "original_score": 85,
  "doubled_score": 170,
  "passed": true,
  "_inserted_id": 1,
  "_rows_affected": 1
}
```

## 🏆 Performance Characteristics

- **Hot Reload**: `<1ms` (atomic pointer swap)
- **Execution Start**: `<5ms` (hash lookup + petgraph)  
- **Memory Overhead**: `O(nodes + edges)` per workflow
- **Concurrent Workflows**: Unlimited (lock-free architecture)
- **Lua Execution**: Sandboxed with 16MB memory limit per script

## 🛡️ Safety Features

### Lua Sandbox
- Memory limits (16MB per execution)
- Script isolation (new Lua instance per execution)
- JSON serialization boundaries
- Error containment with graceful fallbacks

### Database Safety  
- SQL injection prevention via parameterized queries
- Table/column name validation
- Transaction rollback on failures
- Connection pooling with sqlx

### Thread Safety
- Lock-free data structures (`ArcSwap`)
- Immutable workflow definitions  
- Clone-based data passing
- Tokio async runtime for concurrency

## 🔮 Future Extensions

The architecture is designed for easy extension:

- **Additional Node Types**: HTTP, Database, AI/ML nodes
- **Plugin System**: `.so` and `.wasm` plugin loading
- **Distributed Mode**: gRPC/PubSub for multi-node execution
- **Advanced Logic**: WASM runtime, native code execution
- **Monitoring**: Built-in metrics and tracing
- **UI Integration**: React frontend for visual workflow editing

## 📝 Development Notes

### Adding New Node Types
1. Add variant to `NodeType` enum in `workflow/types.rs`
2. Implement execution logic in `runtime/executor.rs`  
3. Add parameter validation and documentation
4. Update API schemas and tests

### Modifying Execution Engine
- Core logic in `runtime/engine.rs`
- Uses `petgraph::algo::toposort` for execution order
- Data flows via `ExecutionContext` between nodes
- Error handling with `anyhow::Result` for detailed errors

### Database Schema Changes
- Workflow schema: `workflow/storage.rs` 
- Data schema: Dynamic (handled by `SimpleTableWriterNode`)
- Migrations: Add to `init_schema()` methods
- Always maintain backward compatibility

## 📚 Dependencies

### Core Runtime
- `axum` - HTTP server framework  
- `tokio` - Async runtime
- `petgraph` - DAG processing
- `arc-swap` - Lock-free data structures

### Data & Serialization
- `serde` + `serde_json` - JSON handling
- `sqlx` - Database operations
- `mlua` - Lua script execution

### Utilities  
- `anyhow` - Error handling
- `tracing` - Structured logging
- `chrono` - Timestamp management

Built with ❤️ for the open-source community. Fast enough for robotics, simple enough for data flows, and open enough for ideas.
