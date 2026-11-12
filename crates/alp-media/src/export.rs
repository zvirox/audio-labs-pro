//! Export settings and format encoders.
//!
//! WAV supports 16/24-bit integer and 32-bit float output with RIFF `INFO`
//! tags. FLAC supports 16/24-bit lossless output with a Vorbis comment block.
//! Both paths resample offline, apply optional TPDF dither, write a sibling
//! temporary file and rename it into place.

use std::path::Path;
use std::str::FromStr;

use alp_core::write_atomic;
use flacenc::component::{BitRepr, MetadataBlockData};
use flacenc::error::Verify;

use crate::dither::{DitherMode, Quantizer};
use crate::error::MediaError;
use crate::pcm::PcmBuffer;
use crate::resample::resample;
use crate::wav::{WavBitDepth, write_wav_with};

/// Output container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// RIFF/WAVE.
    Wav,
    /// Native FLAC stream.
    Flac,
}

impl ExportFormat {
    /// Short label used by the CLI and UI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }
}

impl FromStr for ExportFormat {
    type Err = MediaError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "wav" | "wave" => Ok(Self::Wav),
            "flac" => Ok(Self::Flac),
            other => Err(MediaError::Encode(format!(
                "unknown export format {other:?} (use wav or flac)"
            ))),
        }
    }
}

/// Built-in delivery presets for common export targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportPreset {
    /// Standard podcast master: 44.1 kHz, 16-bit FLAC, triangular dither.
    Podcast,
    /// Standard audiobook master: 44.1 kHz, 16-bit WAV, triangular dither.
    Audiobook,
    /// Broadcast / video voice-over delivery: 48.0 kHz, 24-bit WAV, triangular dither.
    VoiceOver,
}

impl ExportPreset {
    /// Preconfigured export settings for this preset.
    #[must_use]
    pub fn settings(self) -> ExportSettings {
        match self {
            Self::Podcast => ExportSettings {
                format: ExportFormat::Flac,
                bit_depth: WavBitDepth::Int16,
                sample_rate: Some(44_100),
                dither: DitherMode::Triangular,
                metadata: AudioMetadata::default(),
            },
            Self::Audiobook => ExportSettings {
                format: ExportFormat::Wav,
                bit_depth: WavBitDepth::Int16,
                sample_rate: Some(44_100),
                dither: DitherMode::Triangular,
                metadata: AudioMetadata::default(),
            },
            Self::VoiceOver => ExportSettings {
                format: ExportFormat::Wav,
                bit_depth: WavBitDepth::Int24,
                sample_rate: Some(48_000),
                dither: DitherMode::Triangular,
                metadata: AudioMetadata::default(),
            },
        }
    }

    /// Short label used by CLI and UI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Podcast => "podcast",
            Self::Audiobook => "audiobook",
            Self::VoiceOver => "voice-over",
        }
    }
}

impl FromStr for ExportPreset {
    type Err = MediaError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().replace('_', "-").as_str() {
            "podcast" => Ok(Self::Podcast),
            "audiobook" => Ok(Self::Audiobook),
            "voice-over" | "voiceover" => Ok(Self::VoiceOver),
            other => Err(MediaError::Encode(format!(
                "unknown export preset {other:?} (supported: podcast, audiobook, voice-over)"
            ))),
        }
    }
}

/// Basic file tags and synthetic audio provenance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioMetadata {
    /// Track title.
    pub title: Option<String>,
    /// Performing artist.
    pub artist: Option<String>,
    /// Album or project name.
    pub album: Option<String>,
    /// Free-form comment.
    pub comment: Option<String>,
    /// Whether this audio contains AI-synthesized or cloned speech.
    pub synthetic: bool,
    /// Watermark or provenance attribution string for synthetic speech.
    pub watermark: Option<String>,
}

impl AudioMetadata {
    /// Whether every field is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.artist.is_none()
            && self.album.is_none()
            && self.comment.is_none()
            && !self.synthetic
            && self.watermark.is_none()
    }

    /// Build the metadata that marks a file as synthesized speech.
    ///
    /// Every surface that writes generated speech uses this constructor, so
    /// the synthetic-voice tag cannot be dropped by a path that forgot to set
    /// it. The `engine` argument names the engine that actually produced the
    /// audio and is written into both the watermark and the comment.
    #[must_use]
    pub fn synthetic_speech(engine: &str) -> Self {
        Self {
            comment: Some(format!("synthetic_voice: true; engine: {engine}")),
            synthetic: true,
            watermark: Some(format!("Audio Labs Pro ({engine})")),
            ..Self::default()
        }
    }

    fn vorbis_comments(&self) -> Vec<String> {
        let mut comments = Vec::new();
        if let Some(value) = &self.title {
            comments.push(format!("TITLE={value}"));
        }
        if let Some(value) = &self.artist {
            comments.push(format!("ARTIST={value}"));
        }
        if let Some(value) = &self.album {
            comments.push(format!("ALBUM={value}"));
        }
        if let Some(value) = &self.comment {
            comments.push(format!("COMMENT={value}"));
        }
        if self.synthetic {
            comments.push("SYNTHETIC_VOICE=true".into());
            let prov = self
                .watermark
                .as_deref()
                .unwrap_or("Audio Labs Pro Synthesized Speech");
            comments.push(format!("PROVENANCE={prov}"));
        }
        comments
    }

    pub(crate) fn riff_tags(&self) -> Vec<(&'static [u8; 4], std::borrow::Cow<'_, str>)> {
        let mut tags = Vec::new();
        if let Some(value) = &self.title {
            tags.push((b"INAM", std::borrow::Cow::Borrowed(value.as_str())));
        }
        if let Some(value) = &self.artist {
            tags.push((b"IART", std::borrow::Cow::Borrowed(value.as_str())));
        }
        if let Some(value) = &self.album {
            tags.push((b"IPRD", std::borrow::Cow::Borrowed(value.as_str())));
        }
        if let Some(value) = &self.comment {
            if self.synthetic {
                let prov = self
                    .watermark
                    .as_deref()
                    .unwrap_or("Audio Labs Pro Synthesized Speech");
                tags.push((
                    b"ICMT",
                    std::borrow::Cow::Owned(format!(
                        "{value}; synthetic_voice: true; provenance: {prov}"
                    )),
                ));
            } else {
                tags.push((b"ICMT", std::borrow::Cow::Borrowed(value.as_str())));
            }
        } else if self.synthetic {
            let prov = self
                .watermark
                .as_deref()
                .unwrap_or("Audio Labs Pro Synthesized Speech");
            tags.push((
                b"ICMT",
                std::borrow::Cow::Owned(format!("synthetic_voice: true; provenance: {prov}")),
            ));
        }
        if self.synthetic {
            tags.push((
                b"ISFT",
                std::borrow::Cow::Borrowed("Audio Labs Pro (Synthetic Audio)"),
            ));
        }
        tags
    }
}

/// Complete export configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct ExportSettings {
    /// Container to write.
    pub format: ExportFormat,
    /// Integer depth, or float32 for WAV. FLAC rejects float.
    pub bit_depth: WavBitDepth,
    /// Target sample rate, or the source rate when `None`.
    pub sample_rate: Option<u32>,
    /// Dither applied before integer quantization.
    pub dither: DitherMode,
    /// Optional tags.
    pub metadata: AudioMetadata,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            format: ExportFormat::Wav,
            bit_depth: WavBitDepth::Int16,
            sample_rate: None,
            dither: DitherMode::None,
            metadata: AudioMetadata::default(),
        }
    }
}

impl ExportSettings {
    /// Build WAV settings.
    #[must_use]
    pub fn wav(bit_depth: WavBitDepth) -> Self {
        Self {
            format: ExportFormat::Wav,
            bit_depth,
            ..Self::default()
        }
    }

    /// Build FLAC settings, rejecting float depth.
    pub fn flac(bit_depth: WavBitDepth) -> Result<Self, MediaError> {
        if bit_depth == WavBitDepth::Float32 {
            return Err(MediaError::Encode(
                "FLAC export needs 16-bit or 24-bit integer samples".into(),
            ));
        }
        Ok(Self {
            format: ExportFormat::Flac,
            bit_depth,
            ..Self::default()
        })
    }

    /// Build settings from an industry preset.
    #[must_use]
    pub fn preset(preset: ExportPreset) -> Self {
        preset.settings()
    }
}

/// Measurements and format of one completed export.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExportReport {
    /// Frames written.
    pub frames: u64,
    /// Output sample rate.
    pub sample_rate: u32,
    /// Output channels.
    pub channels: u16,
    /// Bits per sample.
    pub bits_per_sample: u16,
    /// Container label.
    pub format: &'static str,
    /// Largest absolute sample before quantization.
    pub peak: f32,
    /// Root-mean-square before quantization.
    pub rms: f32,
}

/// Export a PCM buffer with the requested container and settings.
pub fn export(
    path: impl AsRef<Path>,
    buffer: &PcmBuffer,
    settings: &ExportSettings,
) -> Result<ExportReport, MediaError> {
    if settings.format == ExportFormat::Flac && settings.bit_depth == WavBitDepth::Float32 {
        return Err(MediaError::Encode(
            "FLAC export needs 16-bit or 24-bit integer samples".into(),
        ));
    }
    let target_rate = settings.sample_rate.unwrap_or(buffer.sample_rate());
    let converted = resample(buffer, target_rate)?;
    let report = ExportReport {
        frames: converted.frames(),
        sample_rate: converted.sample_rate(),
        channels: converted.channels(),
        bits_per_sample: settings.bit_depth.bits(),
        format: settings.format.label(),
        peak: converted.peak(),
        rms: converted.rms(),
    };
    match settings.format {
        ExportFormat::Wav => write_wav_with(
            path,
            &converted,
            settings.bit_depth,
            settings.dither,
            &settings.metadata,
        )?,
        ExportFormat::Flac => write_flac(
            path,
            &converted,
            settings.bit_depth,
            settings.dither,
            &settings.metadata,
        )?,
    }
    Ok(report)
}

fn write_flac(
    path: impl AsRef<Path>,
    buffer: &PcmBuffer,
    bit_depth: WavBitDepth,
    dither: DitherMode,
    metadata: &AudioMetadata,
) -> Result<(), MediaError> {
    let bits = match bit_depth {
        WavBitDepth::Int16 => 16,
        WavBitDepth::Int24 => 24,
        WavBitDepth::Float32 => {
            return Err(MediaError::Encode(
                "FLAC export needs 16-bit or 24-bit integer samples".into(),
            ));
        }
    };
    let max = if bits == 16 { 32_767 } else { 8_388_607 };
    let mut quantizer = Quantizer::new(max, dither);
    let samples: Vec<i32> = buffer
        .samples()
        .iter()
        .map(|sample| quantizer.quantize(*sample))
        .collect();
    let source = flacenc::source::MemSource::from_samples(
        &samples,
        usize::from(buffer.channels()),
        bits,
        buffer.sample_rate() as usize,
    );
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|error| MediaError::Encode(format!("FLAC config: {error:?}")))?;
    let mut stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|error| MediaError::Encode(format!("FLAC encode: {error}")))?;
    stream
        .stream_info_mut()
        .set_block_sizes(config.block_size, config.block_size)
        .map_err(|error| MediaError::Encode(format!("FLAC block sizes: {error}")))?;
    if !metadata.is_empty() {
        let comments = metadata.vorbis_comments();
        let payload = vorbis_comment_payload(&comments)?;
        let block = MetadataBlockData::new_unknown(4, &payload)
            .map_err(|error| MediaError::Encode(format!("FLAC metadata: {error}")))?;
        stream.add_metadata_block(block);
    }
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|error| MediaError::Encode(format!("FLAC stream: {error}")))?;
    write_atomic(path.as_ref(), sink.as_slice())?;
    Ok(())
}

fn vorbis_comment_payload(comments: &[String]) -> Result<Vec<u8>, MediaError> {
    let vendor = b"Audio Labs Pro";
    let mut payload = Vec::new();
    let vendor_len = u32::try_from(vendor.len())
        .map_err(|_| MediaError::Encode("FLAC vendor tag is too long".into()))?;
    payload.extend_from_slice(&vendor_len.to_le_bytes());
    payload.extend_from_slice(vendor);
    let count = u32::try_from(comments.len())
        .map_err(|_| MediaError::Encode("too many FLAC comments".into()))?;
    payload.extend_from_slice(&count.to_le_bytes());
    for comment in comments {
        let length = u32::try_from(comment.len())
            .map_err(|_| MediaError::Encode("FLAC comment is too long".into()))?;
        payload.extend_from_slice(&length.to_le_bytes());
        payload.extend_from_slice(comment.as_bytes());
    }
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wav::WavBitDepth;

    fn buffer() -> PcmBuffer {
        PcmBuffer::new(
            44_100,
            2,
            (0..200)
                .map(|index| ((index as f32) * 0.01).sin() * 0.5)
                .collect(),
        )
        .expect("buffer")
    }

    #[test]
    fn format_and_settings_parse() {
        assert_eq!(
            "flac".parse::<ExportFormat>().expect("parse"),
            ExportFormat::Flac
        );
        assert_eq!(
            "tpdf".parse::<DitherMode>().expect("parse"),
            DitherMode::Triangular
        );
        assert!(ExportSettings::flac(WavBitDepth::Float32).is_err());

        assert_eq!(
            "podcast".parse::<ExportPreset>().expect("parse"),
            ExportPreset::Podcast
        );
        assert_eq!(
            "voice-over".parse::<ExportPreset>().expect("parse"),
            ExportPreset::VoiceOver
        );
        let podcast_settings = ExportSettings::preset(ExportPreset::Podcast);
        assert_eq!(podcast_settings.format, ExportFormat::Flac);
        assert_eq!(podcast_settings.sample_rate, Some(44_100));
        assert_eq!(podcast_settings.dither, DitherMode::Triangular);
    }

    #[test]
    fn wav_export_resamples_and_tags() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mix.wav");
        let mut settings = ExportSettings::wav(WavBitDepth::Int16);
        settings.sample_rate = Some(48_000);
        settings.dither = DitherMode::Triangular;
        settings.metadata.title = Some("Take one".into());
        let report = export(&path, &buffer(), &settings).expect("export");
        assert_eq!(report.sample_rate, 48_000);
        assert_eq!(report.format, "wav");
        assert!(path.is_file());
        let bytes = std::fs::read(&path).expect("read");
        assert!(bytes.windows(4).any(|window| window == b"INAM"));
        assert!(bytes.windows(9).any(|window| window == b"Take one\0"));
    }

    #[test]
    fn flac_export_round_trips_through_symphonia() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mix.flac");
        let mut settings = ExportSettings::flac(WavBitDepth::Int16).expect("settings");
        settings.metadata.artist = Some("Studio".into());
        let report = export(&path, &buffer(), &settings).expect("export");
        assert_eq!(report.format, "flac");
        let info = crate::probe(&path).expect("probe");
        assert_eq!(info.sample_rate, 44_100);
        assert_eq!(info.channels, 2);
        let decoded = crate::decode(&path).expect("decode");
        assert_eq!(decoded.frames(), 100);
        assert!(decoded.peak() > 0.4);
    }

    #[test]
    fn float_flac_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("bad.flac");
        let mut settings = ExportSettings::wav(WavBitDepth::Float32);
        settings.format = ExportFormat::Flac;
        assert!(export(&path, &buffer(), &settings).is_err());
    }

    #[test]
    fn export_writes_synthetic_watermark_in_wav_and_flac() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pcm = PcmBuffer::new(48_000, 1, vec![0.0; 4800]).expect("pcm");
        let metadata = AudioMetadata {
            synthetic: true,
            watermark: Some("Audio Labs Pro (qwen3-tts)".into()),
            ..AudioMetadata::default()
        };

        let wav_path = dir.path().join("synth.wav");
        let wav_settings = ExportSettings {
            format: ExportFormat::Wav,
            bit_depth: WavBitDepth::Int16,
            sample_rate: None,
            dither: DitherMode::None,
            metadata: metadata.clone(),
        };
        export(&wav_path, &pcm, &wav_settings).expect("export wav");
        let wav_bytes = std::fs::read(&wav_path).expect("read wav");
        let wav_str = String::from_utf8_lossy(&wav_bytes);
        assert!(wav_str.contains("ISFT"));
        assert!(wav_str.contains("Audio Labs Pro"));
        assert!(wav_str.contains("synthetic_voice: true"));

        let flac_path = dir.path().join("synth.flac");
        let flac_settings = ExportSettings {
            format: ExportFormat::Flac,
            bit_depth: WavBitDepth::Int16,
            sample_rate: None,
            dither: DitherMode::None,
            metadata,
        };
        export(&flac_path, &pcm, &flac_settings).expect("export flac");
        let flac_bytes = std::fs::read(&flac_path).expect("read flac");
        let flac_str = String::from_utf8_lossy(&flac_bytes);
        assert!(flac_str.contains("SYNTHETIC_VOICE=true"));
        assert!(flac_str.contains("PROVENANCE=Audio Labs Pro (qwen3-tts)"));
    }

    #[test]
    fn the_synthetic_speech_helper_names_the_engine() {
        let metadata = AudioMetadata::synthetic_speech("acoustic-clone");
        assert!(metadata.synthetic, "the synthetic flag must be set");
        assert_eq!(
            metadata.watermark.as_deref(),
            Some("Audio Labs Pro (acoustic-clone)")
        );
        assert_eq!(
            metadata.comment.as_deref(),
            Some("synthetic_voice: true; engine: acoustic-clone")
        );
        assert!(!metadata.is_empty(), "a tagged file is never empty");
    }

    #[test]
    fn every_synthetic_export_carries_the_tag() {
        // The helper is the single source of the provenance tag, so a file
        // written through it always carries both the RIFF comment and the
        // software tag.
        let dir = tempfile::tempdir().expect("tempdir");
        let pcm = PcmBuffer::new(48_000, 1, vec![0.0; 4800]).expect("pcm");
        let path = dir.path().join("tagged.wav");
        let settings = ExportSettings {
            format: ExportFormat::Wav,
            bit_depth: WavBitDepth::Int16,
            sample_rate: None,
            dither: DitherMode::None,
            metadata: AudioMetadata::synthetic_speech("builtin-acoustic"),
        };
        export(&path, &pcm, &settings).expect("export");
        let bytes = std::fs::read(&path).expect("read");
        let content = String::from_utf8_lossy(&bytes);
        assert!(content.contains("synthetic_voice: true"));
        assert!(content.contains("engine: builtin-acoustic"));
        assert!(content.contains("ISFT"));
    }
}
