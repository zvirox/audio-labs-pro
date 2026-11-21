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

---

## 4. Live GUI attachment

Enable **Settings → AI control**, choose the permission tier, approve the client,
and copy its fresh session token. Start the same MCP binary with the token only
in its process environment:

```bash
ALP_CONTROL_TOKEN='TOKEN_FROM_THE_RUNNING_UI' \
  alp-mcp --attach --client-id my-local-client
```

Attach mode forwards stdio JSON-RPC to `$XDG_RUNTIME_DIR/alp/control.sock`
instead of opening, creating, or canonicalizing `--workspace`. Use
`--control-socket` only for a deliberate isolated integration fixture. The full
same-version tool/resource/prompt catalog runs in desktop worker threads and
successful edits join the visible project and ordinary undo history.

Subscribe to project changes with:

```json
{"jsonrpc":"2.0","id":3,"method":"resources/subscribe","params":{"uri":"alp://project/active"}}
```

The proxy then emits standard `notifications/resources/updated` plus
`notifications/alp/project_changed` with summary, origin, operation and measured
request-to-live-commit milliseconds. Attached calls are currently serialized
per connection. An edit raced by a human change fails with a revision conflict;
`rollback_session` is refused to protect intervening human edits.

### Native protocol for Rust integrations

The lower-level protocol is version 2. Authentication is
`{"type":"authenticate","protocol_version":2,"token":"...","client_id":"..."}`.
An approved client may send `{"type":"subscribe"}` and receive committed change
events, or `mcp` containing one full JSON-RPC value and `ControlLimits`. Native
frames are capped at 2 MiB. Legacy `project_info`, `track_list`, `track_add` and
`clip_split` calls remain supported for compatibility. Rust callers can use
`alp_api::control::{write_request, read_response}` rather than implementing the
codec.

Stopping AI control rejects queued work, revokes active clients and rotates the
token on the next enable. Never put the token in source control, logs, persisted
client configuration, or command-line arguments.
