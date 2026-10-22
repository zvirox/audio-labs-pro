# Phase 0 spikes

Throwaway evidence code. **Not** part of the shipping workspace created in
Phase 1, never linked into the app. Results live in
`docs/spikes/PHASE0-RESULTS.md`.

```bash
cargo run -p ui-iced-spike --release        # glass + waveform + components
cargo run -p minimal --release              # idle-redraw control experiment
cargo run -p wayland-probe --release        # blur protocol enumeration
cargo run -p audio-spike --release devices
cargo run -p audio-spike --release play 10
cargo run -p audio-spike --release record /tmp/rec.wav 5
cargo run -p audio-spike --release gen /tmp/hour.wav 3600
cargo run -p audio-spike --release peaks /tmp/hour.wav /tmp/hour.peaks
cargo run -p model-spike --release          # Kokoro ONNX (needs the model files)
```

Environment variables used by the UI spike:

| Variable | Effect |
|---|---|
| `ALP_SPIKE_AUTOCLOSE_SECS=N` | close after N seconds and print metrics |
| `ALP_SPIKE_METRICS=1` | print frames/s and CPU% once a second |
| `ALP_SPIKE_START_TAB=waveform\|components` | initial tab |
| `ALP_SPIKE_AUTOSCROLL=1` | start the 60 fps waveform scroll |
| `ALP_SPIKE_TRANSPARENT=1` | skip the faux desktop so the real desktop shows through |

Model spike prerequisites (not committed):

```bash
mkdir -p ~/.local/share/alp/models/kokoro
# model.onnx, tokenizer.json, af_heart.bin from
# https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX
export CMUDICT_PATH=$(ls -d ~/.cargo/registry/src/*/piper-plus-g2p-*/data)/cmudict_data.json
ALP_SPIKE_G2P=1 cargo run -p model-spike --release
```

Models are large (Kokoro ~326 MB) and are downloaded only when a spike needs
them; they are never committed.
