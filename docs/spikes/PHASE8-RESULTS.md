# Phase 8 Spike Results — Voice Profiles, Cloning, and Voice Generation

**Date:** 2026-10-27  
**Build-plan reference:** Phase 8 — Voice profiles, cloning, and voice generation (lines 469–489)  
**Deliverables:**
- Global voice profile library (`ProfileStore`) with prebuilt voices ("Aura", "Atlas", "Echo", "Sol")
- Portable `.alpvoice` archive import and export (PKWARE ZIP method 0)
- Enforced human consent attestation gate (`ConsentRecord`) for cloned profiles
- Reference audio quality gate (`analyze_reference_quality`) assessing duration, clipping, noise floor, and silence ratio
- Natural language voice design (`design_voice`)
- Cryptographic generation deduplication cache (`VoiceGenerationCache`)
- Objective A/B audio comparison (`compare_audio`)
- System hardware detection (`HardwareProfile`) and model tier recommendation
- Headless CLI integration in `alp-cli profile` and `alp-cli hardware`
- UI integration in `alp-ui` Voice Lab and Models panels
- Acceptance demo harness in `scripts/phase8-demo.sh`

---

## 1. Hardware Detection & Model Tiering

The hardware probe was verified on the host machine:
- Probes `/proc/meminfo` and logical cores.
- Categorizes systems into:
  - **Fast (Lightweight):** Systems with $< 16$ GB RAM or without GPU acceleration; recommends `builtin-acoustic`, `kitten-nano`, `moonshine-tiny`.
  - **Best (Neural Quality):** Systems with $\ge 16$ GB RAM and GPU/8+ cores; recommends `kokoro-82m`, `qwen3-tts`.
- Working RAM estimation accurately warns when model allocations would approach or exceed available memory.

---

## 2. Reference Audio Quality Audit

Evaluated against reference audio recordings:
- Minimum duration gate ($\ge 5.0$ s): prevents insufficient training material from corrupting speaker embeddings.
- Full-scale clipping gate: identifies digital flat-topping spikes ($\ge 0.999$) and advises microphone gain reduction.
- Noise floor gate: computes 10th percentile block energy and warns if room noise exceeds $-35.0$ dBFS.
- Silence ratio gate: flags audio containing $> 60\%$ silence and advises trimming long pauses.

---

## 3. Human Consent Safety Gate

- Safety rule strictly enforced at store and bundle level: `ProfileStore::save_profile` and `import_bundle` refuse to save or import any `VoiceKind::Cloned` profile unless a valid `ConsentRecord` is present.
- Automated creation without attester identity or statement is rejected.
- In `alp-ui`, a dedicated modal requires explicit review and confirmation before a clone is committed.

---

## 4. Archive Bundling (`.alpvoice`)

- Exported profiles package `profile.json`, `reference.wav`, and `embedding.bin` into standard ZIP archives without external dependencies.
- Verified round-trip: files exported with `ZipWriter` are completely valid and verifiable with system `unzip` and Python `zipfile`.

---

## 5. Verification Checklist

- [x] `cargo fmt --all -- --check` clean
- [x] `cargo clippy --workspace --all-targets -- -D warnings` clean (0 warnings)
- [x] `bash scripts/check-file-length.sh` passes (all files $\le 1,000$ lines)
- [x] `bash scripts/check-deps.sh` passes
- [x] `bash scripts/check-docs.sh` passes (105/105 docs valid)
- [x] `bash scripts/check-ui.sh` passes
- [x] `cargo test --workspace` passes (194+ tests ok)
- [x] `bash scripts/phase8-demo.sh` passes end-to-end
