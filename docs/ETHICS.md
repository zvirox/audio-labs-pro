# Audio Labs Pro — Voice Cloning Ethics & Safety Policy

Audio Labs Pro provides cutting-edge neural voice synthesis and cloning capabilities. To protect personal identity, prevent disinformation, and foster ethical technology use, Audio Labs Pro enforces strict technical and policy safeguards.

---

## 1. Core Principles

1. **Explicit Human Consent:** Voice cloning must only be performed with the direct, informed consent of the speaker whose voice is being cloned.
2. **Human-Only Attestation Gate:** The consent attestation gate in Audio Labs Pro is strictly human-only. Automated scripts, headless CLI commands, and AI agent interfaces (including MCP) are technically prohibited from bypassing or creating cloned voice profiles without human interaction.
3. **Transparency & Attribution:** By default, all synthesized speech exports carry a `synthetic_voice: true` metadata flag and embedded attribution to ensure media authenticity.
4. **No Unauthorized Celebrity or Public Figure Impersonation:** Audio Labs Pro explicitly forbids distributing or packaging voice profiles of real living individuals without legal authorization.

---

## 2. Technical Safeguards in Audio Labs Pro

- **`ConsentRecord` Enforcement:** Every cloned profile in `alp-voice` must contain a valid `ConsentRecord` detailing:
  - Speaker's full legal name
  - Attester's name and relationship
  - Timestamp of attestation
  - Explicit agreement statement acknowledging permissible use
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
