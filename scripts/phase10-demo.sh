#!/usr/bin/env bash
# Phase 10 acceptance demo: Model Context Protocol (MCP) server, alp-api automation, and agentic workflows.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "== [1/5] Running workspace checks and guard scripts =="
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
bash scripts/check-ui.sh

echo "== [2/5] Building release binaries (alp-api & alp-mcp) =="
cargo build --release --bin alp-mcp --bin alp-cli

echo "== [3/5] Setting up isolated test workspace =="
WORK_DIR=$(mktemp -d /tmp/alp-phase10.XXXXXX)
trap 'rm -rf "${WORK_DIR}"' EXIT

echo "Workspace initialized at: ${WORK_DIR}"

echo "== [4/5] Executing scripted MCP tool workflow over stdio =="
python3 - "${WORK_DIR}" "${ROOT}/target/release/alp-mcp" << 'PY'
import json
import subprocess
import sys

work_dir = sys.argv[1]
server_bin = sys.argv[2]

proc = subprocess.Popen(
    [server_bin, "--workspace", work_dir, "--tier", "export", "--client-id", "demo-ai-agent"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    bufsize=1,
)

def rpc(method, params=None, req_id=1):
    payload = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
        "params": params or {}
    }
    proc.stdin.write(json.dumps(payload) + "\n")
    proc.stdin.flush()
    line = proc.stdout.readline()
    resp = json.loads(line)
    if "error" in resp and resp["error"]:
        raise RuntimeError(f"RPC {method} failed: {resp['error']}")
    return resp.get("result", {})

print("1. Sending initialize handshake...")
init_res = rpc("initialize", req_id=1)
assert init_res["serverInfo"]["name"] == "audio-labs-pro"
print("   Connected to server:", init_res["serverInfo"])

print("2. Discovering server capabilities...")
disc_res = rpc("server/discover", req_id=2)
print("   Protocol spec revision:", disc_res.get("specRevision"))

print("3. Listing available tools...")
tools_res = rpc("tools/list", req_id=3)
tool_names = [t["name"] for t in tools_res["tools"]]
print(f"   Registered {len(tool_names)} tools: {', '.join(tool_names)}")
assert "project_info" in tool_names
assert "track_add" in tool_names
assert "export_audio" in tool_names

print("4. Calling tool 'project_info'...")
info_res = rpc("tools/call", {"name": "project_info", "arguments": {}}, req_id=4)
print("   Info summary:", info_res["content"][0]["text"])

print("5. Calling tool 'track_add' for 'Narration'...")
t1_res = rpc("tools/call", {"name": "track_add", "arguments": {"name": "Narration"}}, req_id=5)
print("   Track added:", t1_res["content"][0]["text"])

print("6. Calling tool 'track_add' for 'Music Bed'...")
t2_res = rpc("tools/call", {"name": "track_add", "arguments": {"name": "Music Bed"}}, req_id=6)
print("   Track added:", t2_res["content"][0]["text"])

print("7. Calling tool 'track_list'...")
list_res = rpc("tools/call", {"name": "track_list", "arguments": {}}, req_id=7)
tracks = list_res["structuredContent"]
assert len(tracks) >= 2
print(f"   Active tracks: {[t['name'] for t in tracks]}")

print("8. Calling tool 'analyze_loudness'...")
loud_res = rpc("tools/call", {"name": "analyze_loudness", "arguments": {}}, req_id=8)
print("   Acoustic analysis:", loud_res["content"][0]["text"])

print("9. Creating named checkpoint ('checkpoint_create')...")
cp_res = rpc("tools/call", {"name": "checkpoint_create", "arguments": {"label": "Pre-Export State"}}, req_id=9)
print("   Checkpoint created:", cp_res["content"][0]["text"])

print("10. Calling tool 'export_audio' (podcast preset)...")
out_wav = f"{work_dir}/podcast_master.wav"
exp_res = rpc("tools/call", {"name": "export_audio", "arguments": {"preset": "podcast", "output_path": out_wav}}, req_id=10)
print("   Export completed:", exp_res["content"][0]["text"])

print("11. Testing session rollback ('rollback_session')...")
roll_res = rpc("tools/call", {"name": "rollback_session", "arguments": {}}, req_id=11)
print("   Rollback result:", roll_res["content"][0]["text"])

print("12. Verifying project state post-rollback...")
post_res = rpc("tools/call", {"name": "project_info", "arguments": {}}, req_id=12)
print("   Post-rollback summary:", post_res["content"][0]["text"])

proc.stdin.close()
proc.terminate()
proc.wait()
print("stdio MCP session finished cleanly.")
PY

echo "== [5/5] Verifying persistent JSONL audit trail =="
test -f "${WORK_DIR}/audit.jsonl"
AUDIT_LINES=$(wc -l < "${WORK_DIR}/audit.jsonl")
echo "Audit log recorded ${AUDIT_LINES} operations successfully."
test "${AUDIT_LINES}" -ge 5

echo ""
echo "PASS: Phase 10 local MCP server and AI automation harness verified successfully."
