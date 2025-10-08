# Mechaway Test Cases

This directory contains organized test cases for the Mechaway workflow automation engine.

## 📁 Structure

```
test-cases/
├── workflows/          # Workflow definition JSON files
├── data/              # Sample data files for testing
└── README.md          # This documentation
```

## 🧪 Test Workflows

### 🚀 **NEW: Blazing Fast Input Pins (Zero Lua Overhead!)**

### `input-pins-test.json` ⚡
**Purpose**: Demonstrate blazing-fast input pins (no FunLogic!)  
**Features**: Direct JSON field extraction via $json.field.path syntax  
**Flow**: `Webhook` → `SimpleTableWriter` (with input pins)  
**Test Command**: 
```bash
# Deploy first
curl -X POST http://localhost:3004/api/workflows \
  -H "Content-Type: application/json" \
  -d @test-cases/workflows/input-pins-test.json

# Test blazing-fast input pins
curl -X POST http://localhost:3004/webhook/input-pins-test/input-test \
  -H "Content-Type: application/json" \
  -d '{
    "user": {"id": 123, "name": "Alice", "email": "alice@test.com"},
    "score": 95,
    "timestamp": "2025-10-07T12:00:00Z"
  }'
```

### `api-polling-workflow.json` 🌐⚡
**Purpose**: CronTrigger → HTTPClient → SimpleTableWriter (input pins!)  
**Features**: Polls [AnimeChan API](https://api.animechan.io/v1/quotes/random) every minute, zero Lua overhead  
**Flow**: `CronTrigger` → `HTTPClient` → `SimpleTableWriter` (with input pins)  
**What it does**: Fetches random anime quotes and stores them with **blazing-fast input pins**:
- `$json.data.content` → quote text
- `$json.data.anime.name` → anime title  
- `$json.data.character.name` → character name
- `$json.data.anime.id` → anime ID (nested number extraction!)

**Deployment**: 
```bash
# Deploy and watch it poll automatically every minute!
curl -X POST http://localhost:3004/api/workflows \
  -H "Content-Type: application/json" \
  -d @test-cases/workflows/api-polling-workflow.json

# Check the anime_quotes table after a few minutes
sqlite3 data/mechaway_data.db "SELECT * FROM anime_quotes ORDER BY id DESC LIMIT 3;"
```

### 📋 **Array Processing & Legacy Tests**

### `simple-array-test.json`
**Purpose**: Basic array-based processing validation  
**Features**: Single webhook → FunLogic with array access  
**Test Command**: 
```bash
curl -X POST http://localhost:3004/webhook/simple-array-test/test \
  -H "Content-Type: application/json" \
  -d '{"name": "Test User", "score": 95, "category": "premium"}'
```

### `test-workflow.json`
**Purpose**: Original POC workflow (webhook → funlogic → tablewriter)  
**Features**: Basic 3-node pipeline with Lua processing  
**Test Command**:
```bash
curl -X POST http://localhost:3004/webhook/poc-grading-workflow/grade \
  -H "Content-Type: application/json" \
  -d '{"student_id": "s123", "score": 85}'
```

### `blog-platform-workflow.json` 
**Purpose**: Complete multi-webhook blogging platform  
**Features**: 4 webhooks (create, list, search, stats) with complex processing  
**Test Commands**:
```bash
# Create post
curl -X POST http://localhost:3004/webhook/blog-platform-workflow/create \
  -d '{"title": "Test Post", "content": "Hello world", "author": "Admin"}'

# List posts  
curl -X POST http://localhost:3004/webhook/blog-platform-workflow/list -d '{}'

# Search posts
curl -X POST http://localhost:3004/webhook/blog-platform-workflow/search \
  -d '{"query": "Test", "limit": 5}'

# Get stats
curl -X POST http://localhost:3004/webhook/blog-platform-workflow/stats -d '{}'
```

### `multi-webhook-workflow.json`
**Purpose**: Multi-entry point workflow demonstration  
**Features**: Multiple webhook triggers with different processing paths  

## 🎯 Testing Guidelines

1. **Deploy workflow first**:
   ```bash
   curl -X POST http://localhost:3004/api/workflows \
     -H "Content-Type: application/json" \
     -d @test-cases/workflows/[workflow-name].json
   ```

2. **Test webhook endpoints** using the commands above

3. **Check server logs** for detailed execution traces

4. **Validate results** match expected array-based processing behavior

## 📊 Expected Behavior

All workflows now use **n8n-style array processing**:
- Single inputs wrapped as `[input]`
- All node outputs are arrays
- Lua scripts access data via `data[1]`, `data[2]`, etc.
- HTTP responses return arrays, even for single results

## 🚀 Performance Targets

- **Simple workflows**: < 2ms execution time
- **Complex workflows**: < 10ms execution time  
- **Database operations**: < 5ms per operation
- **Lua processing**: < 1ms per script execution
