# Phase 6 results — model manager and built-in TTS

Status: Model manager, IPC socket worker (`alp-infer`), built-in TTS, text normalization, and script mode completed.

## Automated/headless checks

`bash scripts/phase6-demo.sh` passed on this machine:

| Check | Result |
|---|---|
| Model catalogue inspection | `builtin-acoustic` (verified/embedded), `kokoro-82m` (327 MB), `kitten-nano` (40.5 MB) |
| License acceptance | Interactive and CLI acceptance round-trip verified |
| Direct TTS generation | 3.130 s of 48 kHz audio generated in 0.042 s (RTF: 0.01, ~48x faster than real-time) |
| First-audio latency | ~42 ms for full sentence, drastically exceeding the <2.0 s acceptance ceiling |
| Out-of-process isolation | `alp-infer` daemon started over Unix domain socket; IPC synthesis passed |
| Multi-line script mode | 3-line narration script rendered, stitched gaplessly (12.60 s audio) |
| Script stem export | 3 stem WAV files rendered and verified |
| Determinism null test | 0.00000000 max difference / 0.00000000 RMS difference between runs |
| Guard scripts | file-length (≤1,000 loc, 0 warnings), check-deps, check-docs (92 docs), check-ui |

## CLI capabilities added

- `alp-cli tts <text> <output.wav> [--voice aura|atlas|echo|sol] [--speed <float>]`: generates speech audio from text.
- `alp-cli model list [--json]`: prints model catalogue, kinds, sizes, licenses, and installation status.
- `alp-cli model verify <id>`: verifies SHA-256 integrity of local model files.
- `alp-cli model accept-license <id>`: records user agreement to model license terms.
- `alp-cli script render <script.json> <output> [--stitch]`: batch renders multi-line scripts to stems or a single stitched narration file.

## UI integration

- Voice Lab panel updated with active voice selector (`Aura`, `Atlas`, `Echo`, `Sol`), live text input field, "Synthesize line" button, and status readout.
- Models panel reflects actual catalogue status (Built-in acoustic, Kokoro-82M, Kitten Nano).

## Known limits

- In-process built-in acoustic tier provides fast, intelligible formant speech; Kokoro-82M ONNX model weights require download for full deep-neural prosody.
- Streaming word-by-word synthesis chunks are returned as complete line buffers.
