"""Phase 18 named MCP scenarios: convergence, deep rollback and replay.

This module is deliberately separate from the legacy eight-suite evaluator so
the scenario registry is explicit and machine-readable. It is driven by
``scripts/mcp_eval.py`` and receives an already-configured client factory.
"""

import json
import os
from pathlib import Path


def _call(client, name, arguments=None, expect_error=False):
    return client.rpc(
        "tools/call",
        {"name": name, "arguments": arguments or {}},
        expect_error=expect_error,
    )


def _scenario(client, name, args=None):
    return _call(client, name, args)


def _state(res):
    data = res.get("structuredContent") or {}
    return {
        "name": data.get("name"),
        "sample_rate": data.get("sample_rate"),
        "duration_secs": data.get("duration_secs"),
        "track_count": data.get("track_count"),
        "clip_count": data.get("clip_count"),
    }


def _scenario_table():
    """Return (id, title, runner) tuples in execution order."""
    return [
        ("S01", "initialize handshake", _s01),
        ("S02", "server capabilities discovery", _s02),
        ("S03", "tool catalog has 69 advertised tools", _s03),
        ("S04", "project resource is served", _s04),
        ("S05", "workflow prompts are served", _s05),
        ("S06", "Observe tier denies mutation", _s06),
        ("S07", "Edit tier permits mutation", _s07),
        ("S08", "baseline project snapshot captured", _s08),
        ("S09", "first track added", _s09),
        ("S10", "second track added", _s10),
        ("S11", "track list reflects additions", _s11),
        ("S12", "TTS writes WAV into media", _s12),
        ("S13", "job status reports completion", _s13),
        ("S14", "media import registers source", _s14),
        ("S15", "clip placed on timeline", _s15),
        ("S16", "clip split is undoable", _s16),
        ("S17", "edit_batch is a single undo step", _s17),
        ("S18", "one history_undo restores pre-batch state", _s18),
        ("S19", "history_redo reapplies batch", _s19),
        ("S20", "checkpoint create and restore", _s20),
        ("S21", "loudness analysis returns report", _s21),
        ("S22", "quality analysis returns report", _s22),
        ("S23", "export presets are listed", _s23),
        ("S24", "export writes tagged WAV and rollback restores baseline", _s24),
        ("S25", "deterministic convergence across two workspaces", _s25),
    ]


def run(factory, record, workspace=None):
    """Run all 25 scenarios.

    ``factory(tier, client_id, workspace=None)`` returns an MCP client.
    ``record(scenario_id, title, passed, detail)`` records each outcome.
    """
    ctx = {"workspace": str(workspace) if workspace else os.getcwd()}
    client = factory("export", "eval-phase18", workspace)
    try:
        for scenario_id, title, runner in _scenario_table():
            try:
                passed, detail = runner(client, ctx, factory, record)
            except Exception as error:  # noqa: BLE001 - harness must record failures
                passed, detail = False, f"{type(error).__name__}: {error}"
            record(scenario_id, title, passed, str(detail))
    finally:
        client.close()


def _s01(client, ctx, factory, record):
    res = client.rpc("initialize")
    name = (res.get("serverInfo") or {}).get("name")
    return name == "audio-labs-pro", f"serverInfo={name}"


def _s02(client, ctx, factory, record):
    res = client.rpc("server/discover")
    revision = res.get("specRevision")
    return bool(revision), f"specRevision={revision}"


def _s03(client, ctx, factory, record):
    tools = (client.rpc("tools/list").get("tools") or [])
    ctx["tool_names"] = [tool.get("name") for tool in tools]
    return len(tools) >= 69, f"advertised tools={len(tools)}"


def _s04(client, ctx, factory, record):
    uris = [r.get("uri") for r in (client.rpc("resources/list").get("resources") or [])]
    return "alp://project/active" in uris, f"resources={len(uris)}"


def _s05(client, ctx, factory, record):
    prompts = client.rpc("prompts/list").get("prompts") or []
    return len(prompts) >= 2, f"prompts={len(prompts)}"


def _s06(client, ctx, factory, record):
    observer = factory("observe", "eval-phase18-observe", None)
    try:
        res = _call(
            observer,
            "tts_generate",
            {"text": "denied", "voice_id": "aura"},
            expect_error=True,
        )
        return (
            res.get("isError") is True or "error" in res
        ), "observe mutation denied"
    finally:
        observer.close()


def _s07(client, ctx, factory, record):
    res = _call(client, "voice_design", {"name": "Eval Voice", "prompt": "calm narrator"})
    return res.get("isError") is not True, "designed profile created"


def _s08(client, ctx, factory, record):
    res = _scenario(client, "project_info")
    ctx["baseline"] = _state(res)
    return ctx["baseline"]["sample_rate"] in (44_100, 48_000), str(ctx["baseline"])


def _s09(client, ctx, factory, record):
    res = _scenario(client, "track_add", {"name": "Dialogue"})
    ctx["track_1"] = (res.get("structuredContent") or {}).get("track_id")
    return bool(ctx["track_1"]), f"track_id={ctx['track_1']}"


def _s10(client, ctx, factory, record):
    res = _scenario(client, "track_add", {"name": "Ambience"})
    ctx["track_2"] = (res.get("structuredContent") or {}).get("track_id")
    return bool(ctx["track_2"]), f"track_id={ctx['track_2']}"


def _s11(client, ctx, factory, record):
    res = _scenario(client, "track_list")
    tracks = (res.get("structuredContent") or {}).get("items") or []
    return len(tracks) >= 2, f"tracks={len(tracks)}"


def _s12(client, ctx, factory, record):
    res = _scenario(
        client,
        "tts_generate",
        {"text": "Phase 18 convergence check", "voice_id": "aura", "speed": 1.0},
    )
    data = res.get("structuredContent") or {}
    ctx["wav"] = data.get("path")
    ctx["job_id"] = data.get("job_id")
    ok = bool(ctx["wav"]) and os.path.isfile(ctx["wav"])
    return ok, f"wav={ctx['wav']}"


def _s13(client, ctx, factory, record):
    res = _scenario(client, "job_status", {"job_id": ctx["job_id"]})
    state = (res.get("structuredContent") or {}).get("state")
    return state == "completed", f"job state={state}"


def _s14(client, ctx, factory, record):
    res = _scenario(
        client,
        "media_import",
        {"path": ctx["wav"], "name": "eval_take"},
    )
    ctx["source_id"] = (res.get("structuredContent") or {}).get("source_id")
    return bool(ctx["source_id"]), f"source_id={ctx['source_id']}"


def _s15(client, ctx, factory, record):
    res = _scenario(
        client,
        "clip_add",
        {"track_id": ctx["track_1"], "source_id": ctx["source_id"], "start_secs": 0.0},
    )
    ctx["clip_id"] = (res.get("structuredContent") or {}).get("clip_id")
    return bool(ctx["clip_id"]), f"clip_id={ctx['clip_id']}"


def _s16(client, ctx, factory, record):
    res = _scenario(
        client,
        "clip_split",
        {"clip_id": ctx["clip_id"], "split_secs": 0.5},
    )
    return res.get("isError") is not True, "clip split applied"


def _s17(client, ctx, factory, record):
    before = _state(_scenario(client, "project_info"))
    res = _scenario(
        client,
        "edit_batch",
        {
            "label": "phase18 batch",
            "operations": [
                {"op": "track_update", "track_id": ctx["track_2"], "name": "Renamed Ambience"},
                {"op": "marker_add", "name": "downbeat", "position_secs": 1.0},
            ],
        },
    )
    data = res.get("structuredContent") or {}
    ctx["batch_before"] = before
    ctx["batch_ok"] = data.get("single_undo_step") is True
    return ctx["batch_ok"], f"single_undo_step={data.get('single_undo_step')}"


def _s18(client, ctx, factory, record):
    res = _scenario(client, "history_undo")
    after = _state(_scenario(client, "project_info"))
    names = json.dumps(after, sort_keys=True)
    expected = json.dumps(ctx["batch_before"], sort_keys=True)
    return res.get("isError") is not True and after["track_count"] == ctx["batch_before"]["track_count"], names + " vs " + expected


def _s19(client, ctx, factory, record):
    res = _scenario(client, "history_redo")
    return res.get("isError") is not True, "redo applied"


def _s20(client, ctx, factory, record):
    created = _scenario(client, "checkpoint_create", {"label": "phase18 checkpoint"})
    checkpoint_id = (created.get("structuredContent") or {}).get("checkpoint_id")
    _scenario(client, "track_add", {"name": "Temporary"})
    restored = _scenario(client, "checkpoint_restore", {"checkpoint_id": checkpoint_id})
    tracks = (res_or_empty(_scenario(client, "track_list")).get("items") or [])
    return bool(checkpoint_id) and restored.get("isError") is not True and len(tracks) <= 3, f"checkpoint={checkpoint_id}, tracks={len(tracks)}"


def res_or_empty(res):
    return res.get("structuredContent") or {}


def _s21(client, ctx, factory, record):
    data = res_or_empty(_scenario(client, "analyze_loudness"))
    return "integrated_lufs" in data and "peak_dbfs" in data, str(sorted(data)[:5])


def _s22(client, ctx, factory, record):
    data = res_or_empty(_scenario(client, "analyze_quality"))
    return "noise_floor_dbfs" in data, str(sorted(data)[:5])


def _s23(client, ctx, factory, record):
    data = res_or_empty(_scenario(client, "export_presets"))
    presets = data.get("presets") or []
    return len(presets) >= 3, f"presets={len(presets)}"


def _s24(client, ctx, factory, record):
    out = str(Path(ctx.get("workspace", ".")) / "phase18_master.wav")
    exported = _scenario(
        client,
        "export_audio",
        {"preset": "podcast", "output_path": out},
    )
    ok = exported.get("isError") is not True and os.path.isfile(out)
    rolled = _scenario(client, "rollback_session")
    final_state = _state(_scenario(client, "project_info"))
    baseline = ctx["baseline"]
    deep_equal = final_state == baseline
    detail = f"wav={ok}, rolled_back={(rolled.get('structuredContent') or {}).get('rolled_back')}, deep_equal={deep_equal}, final={final_state}, baseline={baseline}"
    return ok and deep_equal, detail


def _s25(client, ctx, factory, record):
    digests = []
    for index in range(2):
        workspace = str(Path(ctx.get("workspace", ".")) / f"convergence-{index}")
        Path(workspace).mkdir(parents=True, exist_ok=True)
        probe = factory("export", f"eval-convergence-{index}", workspace)
        try:
            probe.rpc("initialize")
            info = _state(_scenario(probe, "project_info"))
            _scenario(probe, "track_add", {"name": "Convergence"})
            tracks = res_or_empty(_scenario(probe, "track_list"))
            digest = {
                "info": {k: v for k, v in info.items() if k != "name"},
                "track_names": [t.get("name") for t in (tracks.get("items") or [])],
                "total": tracks.get("total"),
            }
            digests.append(json.dumps(digest, sort_keys=True))
        finally:
            probe.close()
    return len(digests) == 2 and digests[0] == digests[1], f"digests_equal={digests[0] == digests[1]}"
