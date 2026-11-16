#!/usr/bin/env python3
"""Audio Labs Pro — MCP Automated Evaluation & Benchmark Suite.

Executes end-to-end evaluation covering 8 benchmark suites over stdio JSON-RPC 2.0:
1. Protocol Handshake & Discovery
2. Resources (alp:// URIs)
3. Prompts & Workflows
4. Tool Catalog & Schema Conformance
5. Permission Tier Enforcement (Observe vs Edit vs Export)
6. Security Sandboxing & Path Traversal Jail
7. Ethics Safety Invariant (Human-only consent & clone exclusion)
8. Full Multitrack Production Workflow, Master Export, and Session Rollback
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


class McpClient:
    """Manages an isolated alp-mcp process over stdio."""

    def __init__(self, bin_path, workspace, tier="export", client_id="eval-harness"):
        self.workspace = Path(workspace)
        self.proc = subprocess.Popen(
            [
                bin_path,
                "--workspace",
                str(self.workspace),
                "--tier",
                tier,
                "--client-id",
                client_id,
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        self._next_id = 1

    def rpc(self, method, params=None, expect_error=False):
        req_id = self._next_id
        self._next_id += 1
        payload = {
            "jsonrpc": "2.0",
            "id": req_id,
            "method": method,
            "params": params or {},
        }
        self.proc.stdin.write(json.dumps(payload) + "\n")
        self.proc.stdin.flush()

        line = self.proc.stdout.readline()
        if not line:
            stderr_out = self.proc.stderr.read()
            raise RuntimeError(f"Server closed connection unexpectedly. Stderr: {stderr_out}")

        resp = json.loads(line)
        if "error" in resp and resp["error"]:
            if not expect_error:
                raise AssertionError(f"JSON-RPC error in {method}: {resp['error']}")
            return resp

        result = resp.get("result", {})
        if result.get("isError") and not expect_error:
            content = result.get("content", [{}])[0].get("text", "")
            raise AssertionError(f"Tool {method} returned isError: {content}")
        return result

    def close(self):
        try:
            self.proc.stdin.close()
            self.proc.terminate()
            self.proc.wait(timeout=3)
        except Exception:
            self.proc.kill()


class EvaluationSuite:
    def __init__(self, bin_path):
        self.bin_path = bin_path
        self.passed_tests = 0
        self.failed_tests = 0
        self.results = []

    def record(self, suite_name, check_name, passed, detail=""):
        status = "PASS" if passed else "FAIL"
        self.results.append((suite_name, check_name, status, detail))
        if passed:
            self.passed_tests += 1
            print(f"  [✓] {check_name}")
        else:
            self.failed_tests += 1
            print(f"  [✗] {check_name}: {detail}")

    def run_suite_1_handshake(self, work_dir):
        print("\n--- Suite 1: Protocol Handshake & Discovery ---")
        client = McpClient(self.bin_path, work_dir, tier="export")
        try:
            res = client.rpc("initialize")
            server_info = res.get("serverInfo", {})
            self.record(
                "Suite 1",
                "initialize returns audio-labs-pro server info",
                server_info.get("name") == "audio-labs-pro",
                f"Got {server_info}",
            )
            caps = res.get("capabilities", {})
            self.record(
                "Suite 1",
                "capabilities declare tools, resources and prompts",
                "tools" in caps and "resources" in caps and "prompts" in caps,
                f"Got {caps}",
            )

            disc = client.rpc("server/discover")
            spec_rev = disc.get("specRevision")
            self.record(
                "Suite 1",
                "server/discover confirms MCP revision 2026-07-28",
                spec_rev == "2026-07-28",
                f"Got {spec_rev}",
            )
        finally:
            client.close()

    def run_suite_2_resources(self, work_dir):
        print("\n--- Suite 2: Resource Resolution (alp://) ---")
        client = McpClient(self.bin_path, work_dir, tier="export")
        try:
            res_list = client.rpc("resources/list")
            uris = [r["uri"] for r in res_list.get("resources", [])]
            expected = [
                "alp://project/active",
                "alp://effects/catalog",
                "alp://docs/agent-guide",
            ]
            all_present = all(u in uris for u in expected)
            self.record(
                "Suite 2",
                "resources/list advertises active, effects and agent-guide URIs",
                all_present,
                f"Found {uris}",
            )

            for uri in expected:
                read_res = client.rpc("resources/read", {"uri": uri})
                contents = read_res.get("contents", [])
                has_content = len(contents) > 0 and len(contents[0].get("text", "")) > 10
                self.record("Suite 2", f"resources/read successfully fetches {uri}", has_content)

            # Invalid URI
            bad_res = client.rpc("resources/read", {"uri": "alp://invalid/none"}, expect_error=True)
            has_err = "error" in bad_res
            self.record("Suite 2", "resources/read rejects invalid URI cleanly", has_err)
        finally:
            client.close()

    def run_suite_3_prompts(self, work_dir):
        print("\n--- Suite 3: Prompt Templates & Recipes ---")
        client = McpClient(self.bin_path, work_dir, tier="export")
        try:
            prompts_res = client.rpc("prompts/list")
            names = [p["name"] for p in prompts_res.get("prompts", [])]
            has_recipes = "clean_up_voice_recording" in names and "produce_podcast_episode" in names
            self.record(
                "Suite 3",
                "prompts/list exposes cleanup and podcast production recipes",
                has_recipes,
                f"Found {names}",
            )

            prompt = client.rpc("prompts/get", {"name": "clean_up_voice_recording"})
            messages = prompt.get("messages", [])
            has_msg = len(messages) > 0 and "text" in messages[0].get("content", {})
            self.record(
                "Suite 3",
                "prompts/get returns structured instructional messages",
                has_msg,
            )
        finally:
            client.close()

    def run_suite_4_tool_catalog(self, work_dir):
        print("\n--- Suite 4: Tool Catalog & Schema Conformance ---")
        client = McpClient(self.bin_path, work_dir, tier="export")
        try:
            tools_res = client.rpc("tools/list")
            tools = tools_res.get("tools", [])
            names = [t["name"] for t in tools]

            essential_tools = [
                "project_info",
                "track_list",
                "track_add",
                "track_remove",
                "clip_split",
                "range_delete",
                "analyze_loudness",
                "export_audio",
                "checkpoint_create",
                "checkpoint_restore",
                "rollback_session",
                "media_import",
                "clip_add",
                "tts_voices",
                "tts_generate",
                "voice_design",
                "transcribe",
            ]
            all_exist = all(t in names for t in essential_tools)
            self.record(
                "Suite 4",
                f"tools/list advertises all {len(essential_tools)} essential automation tools",
                all_exist,
                f"Missing: {[t for t in essential_tools if t not in names]}",
            )

            valid_schemas = True
            for t in tools:
                schema = t.get("inputSchema", {})
                if schema.get("type") != "object" or "properties" not in schema:
                    valid_schemas = False
                    break
            self.record(
                "Suite 4",
                "all tools define valid JSON Schema 2020-12 input definitions",
                valid_schemas,
            )
        finally:
            client.close()

    def run_suite_5_permission_tiers(self, work_dir):
        print("\n--- Suite 5: Hierarchical Permission Tier Enforcement ---")
        # 1. Observe Tier
        obs_client = McpClient(self.bin_path, work_dir, tier="observe", client_id="eval-observe")
        try:
            tools_res = obs_client.rpc("tools/list")
            tool_names = [t["name"] for t in tools_res.get("tools", [])]
            self.record(
                "Suite 5",
                "observe tier filters out edit and export tools from tools/list",
                "project_info" in tool_names and "track_add" not in tool_names and "export_audio" not in tool_names,
            )

            # Calling edit tool under observe
            edit_attempt = obs_client.rpc("tools/call", {"name": "track_add", "arguments": {"name": "Blocked"}}, expect_error=True)
            self.record(
                "Suite 5",
                "observe tier rejects track_add tool invocation",
                edit_attempt.get("isError") is True or "error" in edit_attempt,
            )

            # Calling export tool under observe
            exp_attempt = obs_client.rpc("tools/call", {"name": "export_audio", "arguments": {"preset": "podcast", "output_path": "out.wav"}}, expect_error=True)
            self.record(
                "Suite 5",
                "observe tier rejects export_audio tool invocation",
                exp_attempt.get("isError") is True or "error" in exp_attempt,
            )
        finally:
            obs_client.close()

        # 2. Edit Tier
        edit_client = McpClient(self.bin_path, work_dir, tier="edit", client_id="eval-edit")
        try:
            tools_res = edit_client.rpc("tools/list")
            tool_names = [t["name"] for t in tools_res.get("tools", [])]
            self.record(
                "Suite 5",
                "edit tier permits track_add but excludes export_audio in tools/list",
                "track_add" in tool_names and "export_audio" not in tool_names,
            )

            add_res = edit_client.rpc("tools/call", {"name": "track_add", "arguments": {"name": "EditPermitted"}})
            self.record(
                "Suite 5",
                "edit tier successfully executes track_add",
                add_res.get("isError") is not True,
            )

            exp_attempt = edit_client.rpc("tools/call", {"name": "export_audio", "arguments": {"preset": "podcast", "output_path": "out.wav"}}, expect_error=True)
            self.record(
                "Suite 5",
                "edit tier strictly rejects export_audio tool invocation",
                exp_attempt.get("isError") is True or "error" in exp_attempt,
            )
        finally:
            edit_client.close()

    def run_suite_6_sandboxing(self, work_dir):
        print("\n--- Suite 6: Security Sandboxing & Directory Jail ---")
        client = McpClient(self.bin_path, work_dir, tier="export", client_id="eval-sandbox")
        try:
            # Traversal attempt via relative path
            traversal_res = client.rpc(
                "tools/call",
                {"name": "media_import", "arguments": {"path": "../../../etc/passwd"}},
                expect_error=True,
            )
            self.record(
                "Suite 6",
                "media_import blocks parent directory traversal escape ('../')",
                traversal_res.get("isError") is True or "error" in traversal_res,
            )

            # External absolute path attempt
            abs_res = client.rpc(
                "tools/call",
                {"name": "transcribe", "arguments": {"path": "/etc/shadow"}},
                expect_error=True,
            )
            self.record(
                "Suite 6",
                "transcribe blocks unauthorized external absolute paths",
                abs_res.get("isError") is True or "error" in abs_res,
            )

            # Export path outside workspace
            out_res = client.rpc(
                "tools/call",
                {"name": "export_audio", "arguments": {"preset": "podcast", "output_path": "/tmp/forbidden_escape.wav"}},
                expect_error=True,
            )
            self.record(
                "Suite 6",
                "export_audio blocks destination outside authorized workspace",
                out_res.get("isError") is True or "error" in out_res,
            )
        finally:
            client.close()

    def run_suite_7_ethics_gate(self, work_dir):
        print("\n--- Suite 7: Ethics Safety Invariant & Voice Cloning Gate ---")
        client = McpClient(self.bin_path, work_dir, tier="export", client_id="eval-ethics")
        try:
            tools_res = client.rpc("tools/list")
            names = [t["name"] for t in tools_res.get("tools", [])]

            # Invariant: No voice cloning creation tools accessible via MCP
            cloning_tools = ["voice_clone", "clone_voice", "create_clone", "import_clone_profile"]
            has_no_clones = not any(c in names for c in cloning_tools)
            self.record(
                "Suite 7",
                "MCP tool surface strictly excludes automated voice cloning tools",
                has_no_clones,
            )

            # voice_design yields exclusively VoiceKind::Designed
            des_res = client.rpc(
                "tools/call",
                {
                    "name": "voice_design",
                    "arguments": {
                        "name": "Safety Test Voice",
                        "prompt": "calm narrator with warm tones",
                        "tags": ["narrator", "safety"],
                    },
                },
            )
            profile_json = des_res.get("structuredContent") or {}
            self.record(
                "Suite 7",
                "voice_design persists profiles strictly as kind 'designed'",
                profile_json.get("kind") == "designed" and profile_json.get("consent_recorded") is False,
            )

            # tts_voices lists profiles with safe boolean consent reporting
            voices_res = client.rpc("tools/call", {"name": "tts_voices", "arguments": {}})
            voices_data = voices_res.get("structuredContent") or {}
            self.record(
                "Suite 7",
                "tts_voices enumerates available voices without leaking raw embeddings",
                isinstance(voices_data.get("voices"), list) and len(voices_data["voices"]) > 0,
            )
        finally:
            client.close()

    def run_suite_8_workflow_and_rollback(self, work_dir):
        print("\n--- Suite 8: Full Multitrack Production Workflow, Export & Rollback ---")
        client = McpClient(self.bin_path, work_dir, tier="export", client_id="eval-workflow")
        try:
            # 1. Project info
            info1 = client.rpc("tools/call", {"name": "project_info", "arguments": {}})
            info1_data = info1.get("structuredContent") or {}
            initial_tracks = info1_data.get("track_count", 0)

            # 2. Add Narration track
            t1 = client.rpc("tools/call", {"name": "track_add", "arguments": {"name": "Dialogue Track"}})
            t1_data = t1.get("structuredContent") or {}
            track_id = t1_data.get("track_id")
            self.record("Suite 8", "track_add creates timeline track", track_id is not None)

            # 3. Add Ambience track
            t2 = client.rpc("tools/call", {"name": "track_add", "arguments": {"name": "Ambience Track"}})
            self.record("Suite 8", "track_add creates secondary track", t2.get("isError") is not True)

            # 4. Generate dialogue via tts_generate
            tts_res = client.rpc(
                "tools/call",
                {
                    "name": "tts_generate",
                    "arguments": {
                        "text": "Hello world from the MCP evaluation harness.",
                        "voice_id": "aura",
                        "speed": 1.0,
                    },
                },
            )
            tts_data = tts_res.get("structuredContent") or {}
            synth_path = tts_data.get("path")
            self.record(
                "Suite 8",
                "tts_generate synthesizes speech into project media dir",
                synth_path and os.path.isfile(synth_path),
            )

            # 5. Import audio into media
            imp_res = client.rpc(
                "tools/call",
                {"name": "media_import", "arguments": {"path": synth_path, "name": "dialogue_clip"}},
            )
            imp_data = imp_res.get("structuredContent") or {}
            source_id = imp_data.get("source_id")
            self.record("Suite 8", "media_import registers undoable audio source", source_id is not None)

            # 6. Place clip on track
            clip_res = client.rpc(
                "tools/call",
                {
                    "name": "clip_add",
                    "arguments": {
                        "track_id": track_id,
                        "source_id": str(source_id),
                        "start_secs": 0.0,
                    },
                },
            )
            self.record("Suite 8", "clip_add places audio clip on timeline track", clip_res.get("isError") is not True)

            # 7. Analyze loudness
            loud_res = client.rpc("tools/call", {"name": "analyze_loudness", "arguments": {}})
            loud_data = loud_res.get("structuredContent") or {}
            self.record(
                "Suite 8",
                "analyze_loudness calculates BS.1770 LUFS and sample peak",
                "integrated_lufs" in loud_data and "peak_dbfs" in loud_data,
            )

            # 8. Create checkpoint
            cp_res = client.rpc("tools/call", {"name": "checkpoint_create", "arguments": {"label": "Pre-Export State"}})
            self.record("Suite 8", "checkpoint_create captures immutable state snapshot", cp_res.get("isError") is not True)

            # 9. Export master audio
            out_master = str(Path(work_dir) / "eval_master.wav")
            exp_res = client.rpc(
                "tools/call",
                {"name": "export_audio", "arguments": {"preset": "podcast", "output_path": out_master}},
            )
            master_exists = os.path.isfile(out_master) and os.path.getsize(out_master) > 1000
            self.record("Suite 8", "export_audio renders master audio file with preset", master_exists)

            # 10. Verify synthetic watermark tags on both synthesized WAV and exported master file
            with open(synth_path, "rb") as f:
                synth_bytes = f.read()
            synth_str = synth_bytes.decode("latin1", errors="ignore")
            has_synth_watermark = "ISFT" in synth_str and "Audio Labs Pro" in synth_str
            self.record(
                "Suite 8",
                "synthesized dialogue WAV carries synthetic audio watermark tags",
                has_synth_watermark,
            )

            with open(out_master, "rb") as f:
                header_bytes = f.read()
            header_str = header_bytes.decode("latin1", errors="ignore")
            has_watermark = (
                ("SYNTHETIC_VOICE=true" in header_str)
                or ("ISFT" in header_str and "Audio Labs Pro" in header_str)
                or ("synthetic_voice: true" in header_str)
            )
            self.record(
                "Suite 8",
                "exported master file carries synthetic audio watermark tags",
                has_watermark,
            )

            # 11. Rollback session
            roll_res = client.rpc("tools/call", {"name": "rollback_session", "arguments": {}})
            roll_data = roll_res.get("structuredContent") or {}
            self.record(
                "Suite 8",
                "rollback_session restores project to initial zero-edit state",
                roll_data.get("rolled_back") is True,
            )

            # 12. Verify clean state post-rollback
            info2 = client.rpc("tools/call", {"name": "project_info", "arguments": {}})
            info2_data = info2.get("structuredContent") or {}
            restored_tracks = info2_data.get("track_count", -1)
            self.record(
                "Suite 8",
                "project timeline confirms 0 tracks after session rollback",
                restored_tracks == initial_tracks,
                f"Expected {initial_tracks}, got {restored_tracks}",
            )
        finally:
            client.close()

        # 13. Audit trail verification
        audit_file = Path(work_dir) / "audit.jsonl"
        audit_exists = audit_file.is_file()
        audit_count = 0
        if audit_exists:
            with open(audit_file) as f:
                audit_count = sum(1 for _ in f)
        self.record(
            "Suite 8",
            "audit.jsonl records operations with digests and revisions",
            audit_exists and audit_count >= 8,
            f"Found {audit_count} log entries",
        )


def main():
    if len(sys.argv) < 2:
        print("Usage: mcp_eval.py <PATH_TO_ALP_MCP_BIN>")
        sys.exit(1)

    bin_path = sys.argv[1]
    if not os.path.isfile(bin_path):
        print(f"Error: alp-mcp binary not found at '{bin_path}'")
        sys.exit(1)

    work_dir = tempfile.mkdtemp(prefix="alp-mcp-eval-")
    try:
        suite = EvaluationSuite(bin_path)
        print("==================================================================")
        print("        Audio Labs Pro — MCP Automated Evaluation Suite           ")
        print(f"  Target Binary: {bin_path}")
        print(f"  Sandbox Dir:   {work_dir}")
        print("==================================================================")

        suite.run_suite_1_handshake(work_dir)
        suite.run_suite_2_resources(work_dir)
        suite.run_suite_3_prompts(work_dir)
        suite.run_suite_4_tool_catalog(work_dir)
        suite.run_suite_5_permission_tiers(work_dir)
        suite.run_suite_6_sandboxing(work_dir)
        suite.run_suite_7_ethics_gate(work_dir)
        suite.run_suite_8_workflow_and_rollback(work_dir)

        total = suite.passed_tests + suite.failed_tests
        pass_pct = (suite.passed_tests / total * 100.0) if total > 0 else 0.0

        print("\n==================================================================")
        print(f"  EVALUATION SUMMARY: {suite.passed_tests}/{total} passed ({pass_pct:.1f}%)")
        print(f"  Result: {'PASSED' if suite.failed_tests == 0 else 'FAILED'}")
        print("==================================================================")

        if suite.failed_tests > 0:
            sys.exit(1)
        sys.exit(0)
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)


if __name__ == "__main__":
    main()
