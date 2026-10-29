# Audio Labs Pro — Voice Cloning Ethics & Safety Policy

Audio Labs Pro provides cutting-edge neural voice synthesis and cloning capabilities. To protect personal identity, prevent disinformation, and foster ethical technology use, Audio Labs Pro enforces strict technical and policy safeguards.

---

## 1. Core Principles

1. **Explicit Human Consent:** Voice cloning must only be performed with the direct, informed consent of the speaker whose voice is being cloned.
2. **Human-Only Attestation Gate:** The consent attestation gate in Audio Labs Pro is strictly human-only. Automated AI agent interfaces (including MCP) cannot create cloned voice profiles — the MCP API exposes no cloning tools at all, and the app's consent dialog is human-only. The CLI `profile import` command is technically automatable; it enforces the library's structural consent checks but cannot verify that an attestation is authentic (records are not yet cryptographically signed — see §2).
3. **Transparency & Attribution:** Synthesized speech should be identifiable as such. *Current status:* the `synthetic_voice: true` metadata flag and embedded attribution promised by earlier revisions of this document are **not yet implemented** — exported synthesized audio is written as plain WAV/FLAC with no provenance marker. Adding provenance metadata to the shared export path is a tracked backlog item. Until it ships, do not present Audio Labs Pro output as authentic human recordings.
4. **No Unauthorized Celebrity or Public Figure Impersonation:** Audio Labs Pro explicitly forbids distributing or packaging voice profiles of real living individuals without legal authorization.

---

## 2. Technical Safeguards in Audio Labs Pro

- **`ConsentRecord` Enforcement:** Every cloned profile in `alp-voice` must carry a `ConsentRecord`. Saves and imports reject records that are absent, malformed, or inconsistent with the profile kind (and a non-cloned profile may not carry a consent record at all). A record must detail:
  - Speaker's name
  - Attesting user's name
  - Timestamp of attestation
  - An explicit agreement statement (minimum length)

  *Limit:* records are validated for structure only. They are not cryptographically signed and are not bound to the reference audio, so a hand-written record passes the gate. Signing records and hashing them against the cloned audio is a tracked backlog item; treat cloned profiles from untrusted sources with corresponding caution.
- **Quality Audit Barrier:** Reference audio used for cloning must pass objective acoustic criteria (minimum 5.0s clean speech, no digital clipping, noise floor $\le -35$ dBFS) to prevent low-quality deceptive sound bites.
- **Agent Sandbox Restrictions:** When interacting through automated APIs (such as Phase 10 MCP), profile cloning tools are locked to human approval and cannot be automated unattended.

---

## 3. Acceptable Use Policy

Users of Audio Labs Pro agree **NOT** to use voice synthesis or voice cloning features for:
- Impersonating any individual without their prior express written consent.
- Generating deceptive audio for political manipulation, social engineering, fraud, or harassment.
- Creating non-consensual deepfake audio of any kind.
- Bypassing voice biometric security or authentication systems.

Failure to adhere to this policy violates the ethical terms of service and acceptable-use guidelines of the Audio Labs Pro project.
