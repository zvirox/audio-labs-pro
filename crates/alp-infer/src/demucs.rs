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

/// Description of a compatible ONNX export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DemucsManifest {
    /// ONNX graph path, relative to the model directory.
    pub graph: String,
    /// Waveform input tensor name.
    pub input_name: String,
    /// Four output tensor names, in `STEM_NAMES` order.
    pub output_names: [String; 4],
    /// Sample rate expected by the model.
    pub sample_rate: u32,
    /// Whether the graph needs stereo input (otherwise one channel).
    pub stereo: bool,
}

/// A loaded Demucs-style separation engine.
pub struct DemucsEngine {
    session: Mutex<Session>,
    manifest: DemucsManifest,
}

impl DemucsEngine {
    /// Load a model directory containing a [`MANIFEST_FILE`] and its ONNX graph.
    pub fn load(model_dir: &Path) -> Result<Self, VoiceError> {
        let manifest_path = model_dir.join(MANIFEST_FILE);
        let text = std::fs::read_to_string(&manifest_path).map_err(|error| {
            VoiceError::Backend(format!(
                "missing Demucs manifest {}: {error}",
                manifest_path.display()
            ))
        })?;
        let manifest: DemucsManifest = serde_json::from_str(&text)
            .map_err(|error| VoiceError::Backend(format!("invalid Demucs manifest: {error}")))?;
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
        let mut builder = Session::builder(crate::style_tts2::runtime_environment())
            .map_err(|error| VoiceError::Backend(format!("ONNX builder failed: {error}")))?;
        let session = builder.commit_from_file(&graph).map_err(|error| {
            VoiceError::Backend(format!("loading {}: {error}", graph.display()))
        })?;
        let inputs: Vec<String> = session
            .inputs()
            .iter()
            .map(|input| input.name().to_owned())
            .collect();
        let outputs: Vec<String> = session
            .outputs()
            .iter()
            .map(|output| output.name().to_owned())
            .collect();
        if !inputs.contains(&manifest.input_name) {
            return Err(VoiceError::Backend(format!(
                "Demucs graph is missing input `{}` (found {inputs:?})",
                manifest.input_name
            )));
        }
        for name in &manifest.output_names {
            if !outputs.contains(name) {
                return Err(VoiceError::Backend(format!(
                    "Demucs graph is missing output `{name}` (found {outputs:?})"
                )));
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
        let waveform = Tensor::from_array((vec![1, channels as i64, frames as i64], planar))
            .map_err(|error| VoiceError::Backend(format!("waveform tensor failed: {error}")))?;

        let mut session = self
            .session
            .lock()
            .map_err(|_| VoiceError::Backend("Demucs session lock poisoned".into()))?;
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
        };
        std::fs::write(
            dir.path().join(MANIFEST_FILE),
            serde_json::to_vec(&manifest).expect("json"),
        )
        .expect("write manifest");
        let error = DemucsEngine::load(dir.path()).err().expect("invalid rate");
        assert!(error.to_string().contains("sample_rate"));
    }
}
