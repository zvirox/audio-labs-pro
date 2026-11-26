#!/usr/bin/env bash
# Alpha 6 phase-1 acceptance demo: voice-profile parity, tts_batch and
# audiobook_encode over stdio MCP. Mirrors the style of phase10-demo.sh.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

echo "== [1/3] Running guard scripts =="
bash scripts/check-file-length.sh
bash scripts/check-deps.sh
bash scripts/check-docs.sh
bash scripts/check-ui.sh

echo "== [2/3] Building the MCP server =="
cargo build --release --bin alp-mcp

echo "== [3/3] Scripted MCP smoke over stdio =="
WORK_DIR=$(mktemp -d /tmp/alp-alpha6.XXXXXX)
trap 'rm -rf "${WORK_DIR}"' EXIT
python3 - "${WORK_DIR}" "${ROOT}/target/release/alp-mcp" << 'PY'
import json
import os
import subprocess
import sys

work_dir = sys.argv[1]
server_bin = sys.argv[2]

proc = subprocess.Popen(
    [server_bin, "--workspace", work_dir, "--tier", "export", "--client-id", "demo-agent"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
    text=True, bufsize=1,
)


def rpc(method, params=None, req_id=1):
    proc.stdin.write(json.dumps({
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
        "params": params or {},
    }) + "\n")
    proc.stdin.flush()
    resp = json.loads(proc.stdout.readline())
    if resp.get("error"):
        raise RuntimeError(f"RPC {method} failed: {resp['error']}")
    return resp.get("result", {})


def call(name, args, req_id):
    return rpc("tools/call", {"name": name, "arguments": args}, req_id)


print("1. Initialize handshake...")
rpc("initialize", req_id=1)

print("2. Discovering tools...")
tools_res = rpc("tools/list", req_id=2)
names = [t["name"] for t in tools_res["tools"]]
assert len(names) == 86, f"expected the current 86-tool surface, got {len(names)}"
for tool in ["voice_profile_update", "voice_profile_lock",
             "voice_profile_unlock", "tts_batch", "audiobook_encode"]:
    assert tool in names, f"{tool} missing from the surface"
print(f"   {len(names)} tools registered; the 5 alpha.6 tools are present.")

print("3. voice_design two voices...")
d1 = call("voice_design", {"name": "Alpha", "prompt": "calm narrator"}, 3)["structuredContent"]
d2 = call("voice_design", {"name": "Beta", "prompt": "warm narrator"}, 4)["structuredContent"]
print(f"   Designed: {d1['id']} ({d1['name']}), {d2['id']} ({d2['name']})")

print("4. voice_profile_lock/unlock + update...")
locked = call("voice_profile_lock", {"voice_id": d1["id"]}, 5)["structuredContent"]
assert locked["locked"] is True
err = call("voice_profile_update", {"voice_id": d1["id"], "name": "Renamed"}, 6)
assert err.get("isError"), f"a locked profile must refuse update: {err}"
print("   Locked profile refused update:", err["content"][0]["text"])
call("voice_profile_unlock", {"voice_id": d1["id"]}, 7)
updated = call("voice_profile_update", {"voice_id": d1["id"], "name": "Renamed"}, 8)["structuredContent"]
assert updated["name"] == "Renamed"
print(f"   Unlocked; name -> {updated['name']}")

print("5. tts_batch (2 requests)...")
batch = call("tts_batch", {"requests": [
    {"voice_id": d1["id"], "text": "First line of the batch."},
    {"voice_id": d2["id"], "text": "Second line of the batch."},
]}, 9)
results = batch["structuredContent"]["results"]
assert len(results) == 2
for r in results:
    assert os.path.isfile(r["path"]) and r["duration_secs"] > 0
print(f"   Synthesized {len(results)} clip(s): {[os.path.basename(r['path']) for r in results]}")

print("6. audiobook_encode (2 segments)...")
encode = call("audiobook_encode", {"script_json": json.dumps({"segments": [
    {"text": "Chapter one speaks first.", "voice_id": d1["id"], "gap_secs": 0.1},
    {"text": "Chapter two speaks second.", "voice_id": d2["id"]},
]})}, 10)
report = encode["structuredContent"]
assert os.path.isfile(report["master_path"])
assert report["segment_count"] == 2
assert report["total_duration_secs"] > 0
print(f"   Encoded: track={report['track_id']} master={os.path.basename(report['master_path'])} ({report['segment_count']} segments)")

print("7. Cleaning up...")
proc.stdin.close()
proc.terminate()
proc.wait()
print("stdio MCP smoke finished cleanly.")
PY
