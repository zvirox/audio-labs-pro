//! Script mode data structures, line regeneration and gapless stitching.

use alp_core::CoreError;
use alp_media::{ExportSettings, PcmBuffer, WavBitDepth, export};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Status of speech generation for one script line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStatus {
    /// Not yet generated.
    Pending,
    /// Actively rendering.
    Rendering,
    /// Ready and cached.
    Ready,
    /// Generation failed.
    Failed,
}

/// A single paragraph or spoken line in a script.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScriptLine {
    /// Unique line identifier.
    pub id: String,
    /// Text content to be synthesized.
    pub text: String,
    /// Target voice identifier.
    pub voice_id: String,
    /// Speed multiplier.
    pub speed: f32,
    /// Generation status.
    pub status: LineStatus,
    /// Cached audio PCM buffer (if generated).
    #[serde(skip)]
    pub audio: Option<PcmBuffer>,
}

impl ScriptLine {
    /// Create a new pending script line.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        text: impl Into<String>,
        voice_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            voice_id: voice_id.into(),
            speed: 1.0,
            status: LineStatus::Pending,
            audio: None,
        }
    }

    /// Mark as dirty / pending re-generation when text or parameters change.
    pub fn invalidate(&mut self) {
        self.status = LineStatus::Pending;
        self.audio = None;
    }

    /// Assign rendered audio and mark as ready.
    pub fn set_audio(&mut self, buffer: PcmBuffer) {
        self.audio = Some(buffer);
        self.status = LineStatus::Ready;
    }
}

/// A complete multi-line script for voice-over and podcast narration.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Script {
    /// Title of the script.
    pub title: String,
    /// Default voice used for new lines.
    pub default_voice: String,
    /// Ordered list of script lines.
    pub lines: Vec<ScriptLine>,
}

impl Script {
    /// Create a new empty script.
    #[must_use]
    pub fn new(title: impl Into<String>, default_voice: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            default_voice: default_voice.into(),
            lines: Vec::new(),
        }
    }

    /// Add a line to the end of the script.
    pub fn add_line(&mut self, text: impl Into<String>) -> &mut ScriptLine {
        let index = self.lines.len() + 1;
        let id = format!("line_{index}");
        let voice = self.default_voice.clone();
        self.lines.push(ScriptLine::new(id, text, voice));
        self.lines.last_mut().expect("just pushed")
    }

    /// Find a line by id.
    #[must_use]
    pub fn get_line(&self, id: &str) -> Option<&ScriptLine> {
        self.lines.iter().find(|l| l.id == id)
    }

    /// Find a mutable reference to a line by id.
    pub fn get_line_mut(&mut self, id: &str) -> Option<&mut ScriptLine> {
        self.lines.iter_mut().find(|l| l.id == id)
    }

    /// Update a line's text, invalidating its audio cache only if the text changed.
    pub fn update_text(&mut self, id: &str, new_text: &str) -> bool {
        if let Some(line) = self.get_line_mut(id)
            && line.text != new_text
        {
            line.text = new_text.to_string();
            line.invalidate();
            return true;
        }
        false
    }

    /// Stitch all ready lines into a single gapless PCM buffer with a configurable inter-line pause.
    pub fn stitch(&self, gap_secs: f32) -> Result<PcmBuffer, CoreError> {
        let mut sample_rate = 48_000;
        let mut channels = 1;

        for line in &self.lines {
            if let Some(buf) = &line.audio {
                sample_rate = buf.sample_rate();
                channels = buf.channels();
                break;
            }
        }

        let gap_frames = (gap_secs * sample_rate as f32).max(0.0) as usize;
        let gap_samples = gap_frames * channels as usize;

        let mut combined_samples = Vec::new();

        for (i, line) in self.lines.iter().enumerate() {
            if let Some(buf) = &line.audio {
                combined_samples.extend_from_slice(buf.samples());
                if i + 1 < self.lines.len() && gap_samples > 0 {
                    combined_samples.resize(combined_samples.len() + gap_samples, 0.0);
                }
            }
        }

        PcmBuffer::new(sample_rate, channels, combined_samples)
            .map_err(|e| CoreError::validation(e.to_string()))
    }

    /// Export each line as an individual audio file into `output_dir`.
    pub fn export_stems(&self, output_dir: &Path) -> Result<Vec<std::path::PathBuf>, CoreError> {
        std::fs::create_dir_all(output_dir)?;

        let mut paths = Vec::new();
        let settings = ExportSettings::wav(WavBitDepth::Int24);

        for (idx, line) in self.lines.iter().enumerate() {
            if let Some(buf) = &line.audio {
                let stem_name = format!("{:03}_{}.wav", idx + 1, sanitize_slug(&line.id));
                let stem_path = output_dir.join(stem_name);
                export(&stem_path, buf, &settings).map_err(|e| {
                    CoreError::validation(format!(
                        "failed to export stem {}: {e}",
                        stem_path.display()
                    ))
                })?;
                paths.push(stem_path);
            }
        }

        Ok(paths)
    }
}

fn sanitize_slug(input: &str) -> String {
    let mut s = String::new();
    for c in input.chars() {
        if c.is_alphanumeric() || c == '_' || c == '-' {
            s.push(c);
        } else {
            s.push('_');
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalidating_line_clears_audio_and_preserves_others() {
        let mut script = Script::new("Narration", "voice_1");
        script.add_line("First line");
        script.add_line("Second line");

        let buf = PcmBuffer::new(48000, 1, vec![0.1; 4800]).unwrap();
        script.lines[0].set_audio(buf.clone());
        script.lines[1].set_audio(buf);

        assert_eq!(script.lines[0].status, LineStatus::Ready);
        assert_eq!(script.lines[1].status, LineStatus::Ready);

        // Edit text of line 0
        script.update_text("line_1", "Updated first line");
        assert_eq!(script.lines[0].status, LineStatus::Pending);
        assert!(script.lines[0].audio.is_none());

        // Line 1 is unaffected
        assert_eq!(script.lines[1].status, LineStatus::Ready);
        assert!(script.lines[1].audio.is_some());
    }

    #[test]
    fn stitching_combines_buffers_with_gap() {
        let mut script = Script::new("Test", "v1");
        script.add_line("L1");
        script.add_line("L2");

        let b1 = PcmBuffer::new(48000, 1, vec![0.5; 4800]).unwrap();
        let b2 = PcmBuffer::new(48000, 1, vec![0.8; 4800]).unwrap();

        script.lines[0].set_audio(b1);
        script.lines[1].set_audio(b2);

        // 0.1s gap at 48000 Hz = 4800 samples
        let stitched = script.stitch(0.1).expect("stitch");
        assert_eq!(stitched.frames(), 14400); // 4800 + 4800 gap + 4800
    }
}
