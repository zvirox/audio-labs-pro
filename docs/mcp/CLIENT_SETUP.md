# Audio Labs Pro — MCP Client Setup

Guide for connecting AI assistants and IDEs to the local `alp-mcp` server.

---

## 1. Claude Desktop Configuration

Add the following to your `claude_desktop_config.json` (typically at `~/.config/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "audio-labs-pro": {
      "command": "/path/to/audio-labs-pro/target/release/alp-mcp",
      "args": [
        "--workspace",
        "/home/username/Projects/my-podcast",
        "--tier",
        "export"
      ]
    }
  }
}
```

---

## 2. Cursor / VSCode Configuration

Add the server entry to your project's `.cursor/mcp.json` or `.vscode/mcp.json`:

```json
{
  "mcpServers": {
    "alp": {
      "command": "alp-mcp",
      "args": ["--workspace", "${workspaceFolder}", "--tier", "export"]
    }
  }
}
```

---

## 3. Command Line Testing

You can interact with `alp-mcp` directly via standard input:

```bash
cargo run --release --bin alp-mcp -- --workspace /tmp/alp-workspace
```

Send an initialization handshake:
```json
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

List available tools:
```json
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```
