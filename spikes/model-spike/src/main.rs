//! Phase 0 model spike A: run Kokoro-82M through ONNX Runtime from Rust and
//! measure first-audio latency and real-time factor.
//!
//! Usage: model-spike <model-dir>
//!   <model-dir>/model.onnx
//!   <model-dir>/tokenizer.json
//!   <model-dir>/af_heart.bin   (style vectors, 510 × 256 f32)
//!
//! Phonemes are supplied as a fixed IPA string: the shipped app needs a text →
//! phoneme step, which is an open licensing question (espeak-ng is GPL).

use std::time::Instant;

use ort::session::Session;
use ort::value::Tensor;

const SAMPLE_RATE: u32 = 24_000;

/// "Hello from Audio Labs Pro." in Kokoro's IPA alphabet (fallback when no G2P).
const IPA: &str = "həlˈoʊ fɹʌm ˈɔːdioʊ læb pɹˈoʊ";
const TEXT: &str = "Hello from Audio Labs Pro.";

fn phonemize(text: &str) -> Result<String, Box<dyn std::error::Error>> {
    use piper_plus_g2p::Phonemizer;
    use piper_plus_g2p::english::EnglishPhonemizer;

    let phonemizer = EnglishPhonemizer::new()?;
    let (tokens, _prosody) = phonemizer.phonemize_with_prosody(text)?;
    Ok(tokens.concat())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args().nth(1).unwrap_or_else(|| {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.local/share/alp/models/kokoro")
    });
    let path = std::path::Path::new(&dir);

    let started = Instant::now();
    let environment = ort::init().with_name("alp-model-spike").build()?;
    let mut session = Session::builder(&environment)?.commit_from_file(path.join("model.onnx"))?;
    println!("session loaded in {:?}", started.elapsed());
    for input in session.inputs() {
        println!("  input {} · {:?}", input.name(), input.dtype());
    }
    for output in session.outputs() {
        println!("  output {} · {:?}", output.name(), output.dtype());
    }

    let ipa = if std::env::var("ALP_SPIKE_G2P").is_ok() {
        let ipa = phonemize(TEXT)?;
        println!("g2p: {TEXT:?} -> {ipa:?}");
        ipa
    } else {
        IPA.to_string()
    };

    // Kokoro's tokenizer.json is a character vocabulary wrapped by `$` BOS/EOS.
    // Parsing the 115-entry vocab directly avoids pulling the full tokenizers
    // stack into the app.
    let tokenizer_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path.join("tokenizer.json"))?)?;
    let vocab = tokenizer_json["model"]["vocab"]
        .as_object()
        .ok_or("tokenizer vocab missing")?;
    let mut ids: Vec<i64> = vec![0];
    let mut skipped = String::new();
    for character in ipa.chars() {
        match vocab.get(character.to_string().as_str()) {
            Some(id) => ids.push(id.as_u64().unwrap_or(0) as i64),
            None => skipped.push(character),
        }
    }
    ids.push(0);
    println!("tokens={} skipped={skipped:?}", ids.len());

    let voices = std::fs::read(path.join("af_heart.bin"))?;
    let style_count = voices.len() / (256 * 4);
    println!("style vectors in voice file: {style_count}");
    let style_index = ids.len().min(style_count.saturating_sub(1));
    let style: Vec<f32> = voices[style_index * 256 * 4..(style_index + 1) * 256 * 4]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|bytes| f32::from_le_bytes(*bytes))
        .collect();

    let tokens = ids.len() as i64;

    for run in 1..=3 {
        let input_ids = Tensor::from_array((vec![1, tokens], ids.clone()))?;
        let style = Tensor::from_array((vec![1, 256], style.clone()))?;
        let speed = Tensor::from_array((vec![1], vec![1.0f32]))?;
        let started = Instant::now();
        let outputs = session.run(ort::inputs![
            "input_ids" => input_ids,
            "style" => style,
            "speed" => speed,
        ])?;
        let elapsed = started.elapsed();

        let (shape, audio) = outputs[0].try_extract_tensor::<f32>()?;
        let samples = audio.len();
        let seconds = samples as f64 / SAMPLE_RATE as f64;
        println!(
            "run {run}: {:?} · {samples} samples ({seconds:.2}s) · shape {shape:?}",
            elapsed
        );
        println!(
            "  RTF = {:.3} (inference / audio duration)",
            elapsed.as_secs_f64() / seconds
        );

        if run == 1 {
            write_wav(audio)?;
        }
    }

    Ok(())
}

fn write_wav(audio: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
    let samples = audio.len();
    let peak = audio.iter().fold(0.0f32, |acc, v| acc.max(v.abs()));
    let rms = (audio.iter().map(|v| (v * v) as f64).sum::<f64>() / samples as f64).sqrt();
    println!("peak={peak:.4} rms={rms:.4}");

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let out = std::env::var("ALP_SPIKE_TTS_OUT").unwrap_or_else(|_| "/tmp/kokoro-spike.wav".into());
    let mut writer = hound::WavWriter::create(&out, spec)?;
    for sample in audio {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;
    println!("wrote {out}");

    Ok(())
}
