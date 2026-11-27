//! Four-stem Demucs-style separation through ONNX Runtime.
//!
//! The exact export (graph path, tensor names, sample rate) is described by a
//! `demucs.json` manifest inside the model directory. This keeps the runner
//! honest: it does not assume a particular public ONNX export's tensor names,
//! and refuses to load a graph that does not match the manifest.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use alp_media::{PcmBuffer, resample};
use alp_voice::VoiceError;
use ort::session::Session;
use ort::value::Tensor;
use serde::{Deserialize, Serialize};

/// Catalogue id used for the user-supplied Demucs model directory.
pub const MODEL_ID: &str = "demucs-4stem";
/// Default manifest filename inside the model directory.
pub const MANIFEST_FILE: &str = "demucs.json";
/// Stem order returned by [`DemucsEngine::separate`].
pub const STEM_NAMES: [&str; 4] = ["vocals", "drums", "bass", "other"];

/// Optional description when a graph packs all four stems into one rank-4 tensor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackedStemOutput {
    /// Output tensor name producing shape `[1, 4, C, N]`.
    pub output_name: String,
    /// Ordered stem names along axis 1 (e.g. `["drums", "bass", "other", "vocals"]`).
    pub source_order: [String; 4],
    /// Fixed input chunk length in frames (if the ONNX graph expects overlap-add chunking).
    pub segment_frames: Option<usize>,
}

/// Description of a compatible ONNX export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DemucsManifest {
    /// ONNX graph path, relative to the model directory.
    pub graph: String,
    /// Waveform input tensor name.
    pub input_name: String,
    /// Four output tensor names, in `STEM_NAMES` order (or repeated packed tensor name).
    pub output_names: [String; 4],
    /// Sample rate expected by the model.
    pub sample_rate: u32,
    /// Whether the graph needs stereo input (otherwise one channel).
    pub stereo: bool,
    /// Packed 4-stem output tensor metadata (for single-output HTDemucs exports).
    #[serde(default)]
    pub packed: Option<PackedStemOutput>,
}

/// Default HTDemucs 4-stem ONNX filenames installed by `ModelManager`.
const DEFAULT_HTDEMUCS_GRAPHS: [&str; 2] = ["htdemucs_fp16weights.onnx", "htdemucs.onnx"];
/// Standard HTDemucs segment duration (7.8 s @ 44.1 kHz = 343,980 frames).
const HTDEMUCS_SEGMENT_FRAMES: usize = 343_980;

/// Build the built-in HTDemucs manifest if a standard ONNX graph is installed.
fn default_htdemucs_manifest(model_dir: &Path) -> Option<DemucsManifest> {
    let graph = DEFAULT_HTDEMUCS_GRAPHS
        .iter()
        .find(|name| model_dir.join(name).is_file())?;
    Some(DemucsManifest {
        graph: (*graph).to_owned(),
        input_name: "mix".into(),
        output_names: ["stems", "stems", "stems", "stems"].map(str::to_owned),
        sample_rate: 44_100,
        stereo: true,
        packed: Some(PackedStemOutput {
            output_name: "stems".into(),
            source_order: ["drums", "bass", "other", "vocals"].map(str::to_owned),
            segment_frames: Some(HTDEMUCS_SEGMENT_FRAMES),
        }),
    })
}

/// A loaded Demucs-style separation engine.
pub struct DemucsEngine {
    session: Mutex<Session>,
    manifest: DemucsManifest,
}

impl DemucsEngine {
    /// Load a model directory containing a [`MANIFEST_FILE`] or standard HTDemucs ONNX file.
    pub fn load(model_dir: &Path) -> Result<Self, VoiceError> {
        let manifest_path = model_dir.join(MANIFEST_FILE);
        let manifest = if manifest_path.is_file() {
            let text = std::fs::read_to_string(&manifest_path).map_err(|error| {
                VoiceError::Backend(format!(
                    "missing Demucs manifest {}: {error}",
                    manifest_path.display()
                ))
            })?;
            serde_json::from_str::<DemucsManifest>(&text)
                .map_err(|error| VoiceError::Backend(format!("invalid Demucs manifest: {error}")))?
        } else if let Some(auto) = default_htdemucs_manifest(model_dir) {
            auto
        } else {
            return Err(VoiceError::Backend(format!(
                "missing Demucs manifest {}: install `demucs-4stem` via `alp-cli model download demucs-4stem` or provide demucs.json",
                manifest_path.display()
            )));
        };
        if manifest.sample_rate == 0 {
            return Err(VoiceError::Backend(
                "Demucs manifest sample_rate must be non-zero".into(),
            ));
        }
        let graph = model_dir.join(&manifest.graph);
        if !graph.is_file() {
            return Err(VoiceError::Backend(format!(
                "Demucs graph {} is missing",
                graph.display()
            )));
        }
        crate::style_tts2::ensure_runtime_environment();
        let builder = Session::builder()
            .map_err(|error| VoiceError::Backend(format!("ONNX builder failed: {error}")))?;
        let session = builder.commit_from_file(&graph).map_err(|error| {
            VoiceError::Backend(format!("loading {}: {error}", graph.display()))
        })?;
        let inputs: Vec<String> = session
            .inputs
            .iter()
            .map(|input| input.name.clone())
            .collect();
        let outputs: Vec<String> = session
            .outputs
            .iter()
            .map(|output| output.name.clone())
            .collect();
        if !inputs.contains(&manifest.input_name) {
            return Err(VoiceError::Backend(format!(
                "Demucs graph is missing input `{}` (found {inputs:?})",
                manifest.input_name
            )));
        }
        if let Some(packed) = &manifest.packed {
            if !outputs.contains(&packed.output_name) {
                return Err(VoiceError::Backend(format!(
                    "Demucs graph is missing packed output `{}` (found {outputs:?})",
                    packed.output_name
                )));
            }
        } else {
            for name in &manifest.output_names {
                if !outputs.contains(name) {
                    return Err(VoiceError::Backend(format!(
                        "Demucs graph is missing output `{name}` (found {outputs:?})"
                    )));
                }
            }
        }
        Ok(Self {
            session: Mutex::new(session),
            manifest,
        })
    }

    /// Load from the `ALP_DEMUCS_DIR` environment override, when set.
    pub fn from_environment() -> Option<Result<Self, VoiceError>> {
        std::env::var_os("ALP_DEMUCS_DIR").map(|dir| Self::load(Path::new(&PathBuf::from(dir))))
    }

    /// Manifest this engine was loaded with.
    #[must_use]
    pub fn manifest(&self) -> &DemucsManifest {
        &self.manifest
    }

    /// Separate a buffer into vocals, drums, bass and other stems.
    ///
    /// Input is resampled to the manifest rate and arranged as `[1, C, N]`;
    /// output tensors are expected in the same layout. The returned stems are
    /// resampled back to the caller's sample rate.
    pub fn separate(&self, buffer: &PcmBuffer) -> Result<[PcmBuffer; 4], VoiceError> {
        let source_rate = buffer.sample_rate();
        let model_rate = self.manifest.sample_rate;
        let prepared = if source_rate == model_rate {
            std::borrow::Cow::Borrowed(buffer)
        } else {
            std::borrow::Cow::Owned(
                resample(buffer, model_rate)
                    .map_err(|error| VoiceError::Backend(format!("resample failed: {error}")))?,
            )
        };
        let channels = if self.manifest.stereo { 2 } else { 1 };
        let frames = prepared.frames() as usize;
        if frames == 0 {
            return Err(VoiceError::Backend(
                "cannot separate an empty buffer".into(),
            ));
        }
        let mut interleaved = Vec::with_capacity(frames * channels);
        let source = prepared.samples();
        let source_channels = usize::from(prepared.channels());
        for frame in 0..frames {
            for channel in 0..channels {
                let index = if channel < source_channels {
                    frame * source_channels + channel
                } else {
                    frame * source_channels
                };
                interleaved.push(source[index]);
            }
        }

        // Transpose interleaved [frame, channel] to planar [channel, frame].
        let mut planar = vec![0.0f32; frames * channels];
        for frame in 0..frames {
            for channel in 0..channels {
                planar[channel * frames + frame] = interleaved[frame * channels + channel];
            }
        }

        let mut session = self
            .session
            .lock()
            .map_err(|_| VoiceError::Backend("Demucs session lock poisoned".into()))?;

        if let Some(packed) = &self.manifest.packed {
            return self.separate_packed(
                &mut session,
                packed,
                &planar,
                channels,
                frames,
                model_rate,
                source_rate,
            );
        }

        let waveform = Tensor::from_array((vec![1, channels as i64, frames as i64], planar))
            .map_err(|error| VoiceError::Backend(format!("waveform tensor failed: {error}")))?;

        let outputs = session
            .run(ort::inputs![self.manifest.input_name.as_str() => waveform])
            .map_err(|error| VoiceError::Backend(format!("Demucs inference failed: {error}")))?;

        let mut stems = Vec::with_capacity(4);
        for (index, name) in self.manifest.output_names.iter().enumerate() {
            let (shape, data) = outputs[index]
                .try_extract_tensor::<f32>()
                .map_err(|error| {
                    VoiceError::Backend(format!("reading Demucs output {name}: {error}"))
                })?;
            if shape.len() != 3 {
                return Err(VoiceError::Backend(format!(
                    "Demucs output {name} must be rank 3 [1, C, N], got {shape:?}"
                )));
            }
            let output_channels = shape[1] as usize;
            let output_frames = shape[2] as usize;
            if data.len() < output_channels * output_frames {
                return Err(VoiceError::Backend(format!(
                    "Demucs output {name} is shorter than its shape"
                )));
            }
            let mut stem_interleaved = vec![0.0f32; output_frames * channels];
            for frame in 0..output_frames.min(frames) {
                for channel in 0..channels {
                    let source_channel = channel.min(output_channels.saturating_sub(1));
                    stem_interleaved[frame * channels + channel] =
                        data[source_channel * output_frames + frame];
                }
            }
            let pcm = PcmBuffer::new(model_rate, channels as u16, stem_interleaved)
                .map_err(|error| VoiceError::Backend(format!("stem buffer failed: {error}")))?;
            let restored = if source_rate == model_rate {
                pcm
            } else {
                resample(&pcm, source_rate).map_err(|error| {
                    VoiceError::Backend(format!("stem resample failed: {error}"))
                })?
            };
            stems.push(restored);
        }

        Ok([
            stems.remove(0),
            stems.remove(0),
            stems.remove(0),
            stems.remove(0),
        ])
    }

    #[allow(clippy::too_many_arguments)]
    fn separate_packed(
        &self,
        session: &mut Session,
        packed: &PackedStemOutput,
        planar: &[f32],
        channels: usize,
        total_frames: usize,
        model_rate: u32,
        source_rate: u32,
    ) -> Result<[PcmBuffer; 4], VoiceError> {
        let seg_len = packed.segment_frames.unwrap_or(total_frames).max(1);
        let overlap = (seg_len / 4).min(seg_len.saturating_sub(1));
        let stride = (seg_len - overlap).max(1);
        let window = overlap_window(seg_len, overlap);
        let mut accum = vec![0.0f32; 4 * channels * total_frames];
        let mut weights = vec![0.0f32; total_frames];

        let mut start = 0usize;
        while start < total_frames {
            let end = (start + seg_len).min(total_frames);
            let valid = end - start;
            let mut chunk = vec![0.0f32; channels * seg_len];
            for ch in 0..channels {
                chunk[ch * seg_len..ch * seg_len + valid]
                    .copy_from_slice(&planar[ch * total_frames + start..ch * total_frames + end]);
            }
            let tensor = Tensor::from_array((vec![1, channels as i64, seg_len as i64], chunk))
                .map_err(|error| VoiceError::Backend(format!("chunk tensor failed: {error}")))?;
            let outputs = session
                .run(ort::inputs![self.manifest.input_name.as_str() => tensor])
                .map_err(|error| {
                    VoiceError::Backend(format!("Demucs inference failed: {error}"))
                })?;
            let (shape, data) = outputs[packed.output_name.as_str()]
                .try_extract_tensor::<f32>()
                .map_err(|error| VoiceError::Backend(format!("reading Demucs stems: {error}")))?;
            if shape.len() != 4 || shape[1] != 4 {
                return Err(VoiceError::Backend(format!(
                    "Demucs packed output `{}` must be rank 4 [1, 4, C, N], got {shape:?}",
                    packed.output_name
                )));
            }
            let out_ch = shape[2] as usize;
            let out_frames = shape[3] as usize;
            let copy_len = valid.min(out_frames);
            for src in 0..4 {
                let src_base = src * out_ch * out_frames;
                for ch in 0..channels {
                    let use_ch = ch.min(out_ch.saturating_sub(1));
                    let dst_base = (src * channels + ch) * total_frames;
                    let ch_off = src_base + use_ch * out_frames;
                    for i in 0..copy_len {
                        accum[dst_base + start + i] += data[ch_off + i] * window[i];
                    }
                }
            }
            for i in 0..copy_len {
                weights[start + i] += window[i];
            }
            if end == total_frames {
                break;
            }
            start += stride;
        }

        let mut canonical = Vec::with_capacity(4);
        for target_name in STEM_NAMES {
            let src_idx = packed
                .source_order
                .iter()
                .position(|s| s == target_name)
                .ok_or_else(|| {
                    VoiceError::Backend(format!(
                        "packed source_order missing `{target_name}`: {:?}",
                        packed.source_order
                    ))
                })?;
            let mut interleaved = vec![0.0f32; total_frames * channels];
            for frame in 0..total_frames {
                let w = weights[frame].max(1e-8);
                for ch in 0..channels {
                    let val = accum[(src_idx * channels + ch) * total_frames + frame] / w;
                    interleaved[frame * channels + ch] = val;
                }
            }
            let pcm = PcmBuffer::new(model_rate, channels as u16, interleaved)
                .map_err(|error| VoiceError::Backend(format!("stem buffer failed: {error}")))?;
            canonical.push(if source_rate == model_rate {
                pcm
            } else {
                resample(&pcm, source_rate).map_err(|error| {
                    VoiceError::Backend(format!("stem resample failed: {error}"))
                })?
            });
        }
        Ok([
            canonical.remove(0),
            canonical.remove(0),
            canonical.remove(0),
            canonical.remove(0),
        ])
    }
}

fn overlap_window(len: usize, overlap: usize) -> Vec<f32> {
    let mut w = vec![1.0f32; len];
    if overlap > 1 {
        let denom = (overlap - 1) as f32;
        for i in 0..overlap {
            let ramp = i as f32 / denom;
            w[i] = ramp;
            w[len - 1 - i] = ramp;
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_manifest_is_reported() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = DemucsEngine::load(dir.path())
            .err()
            .expect("missing manifest");
        assert!(error.to_string().contains("missing Demucs manifest"));
    }

    #[test]
    fn manifest_requires_nonzero_sample_rate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = DemucsManifest {
            graph: "model.onnx".into(),
            input_name: "waveform".into(),
            output_names: STEM_NAMES.map(str::to_owned),
            sample_rate: 0,
            stereo: false,
            packed: None,
        };
        std::fs::write(
            dir.path().join(MANIFEST_FILE),
            serde_json::to_vec(&manifest).expect("json"),
        )
        .expect("write manifest");
        let error = DemucsEngine::load(dir.path()).err().expect("invalid rate");
        assert!(error.to_string().contains("sample_rate"));
    }

    #[test]
    fn default_htdemucs_manifest_resolves_when_fp16_onnx_exists() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(default_htdemucs_manifest(dir.path()).is_none());
        std::fs::write(dir.path().join("htdemucs_fp16weights.onnx"), b"stub").expect("stub onnx");
        let manifest = default_htdemucs_manifest(dir.path()).expect("auto manifest");
        assert_eq!(manifest.graph, "htdemucs_fp16weights.onnx");
        assert_eq!(manifest.sample_rate, 44_100);
        assert!(manifest.stereo);
        assert_eq!(
            manifest.packed.as_ref().expect("packed").source_order,
            ["drums", "bass", "other", "vocals"]
        );
    }

    #[test]
    fn overlap_window_tapers_ends_symmetrically() {
        let w = overlap_window(12, 3);
        assert_eq!(w.len(), 12);
        assert!((w[0] - 0.0).abs() < 1e-6);
        assert!((w[1] - 0.5).abs() < 1e-6);
        assert!((w[5] - 1.0).abs() < 1e-6);
        assert!((w[10] - 0.5).abs() < 1e-6);
        assert!((w[11] - 0.0).abs() < 1e-6);
    }
}
