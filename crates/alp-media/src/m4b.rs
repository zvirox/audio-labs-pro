//! Native M4B audiobook muxer with AAC-LC audio, chapters, cover art and tags.
//!
//! AAC frames are produced by the pure-Rust `oxideav-aac` encoder and placed in
//! an ISO-BMFF container via the `mp4` crate. A post-pass injects `chpl`, `covr`
//! and `ilst` boxes into `moov`, which the writer does not expose.

use std::io::Cursor;
use std::path::Path;

use alp_core::write_atomic;
use bytes::Bytes;
use mp4::{
    AacConfig, AudioObjectType, ChannelConfig, MediaConfig, Mp4Config, Mp4Sample, Mp4Writer,
    SampleFreqIndex, TrackConfig,
};
use oxideav_aac::adts::AdtsHeader;
use oxideav_aac::encoder::{EncoderConfig, StreamEncoder};
use serde::{Deserialize, Serialize};

use crate::error::MediaError;
use crate::pcm::PcmBuffer;
use crate::resample::resample;

/// AAC-LC frame size in samples per channel.
const AAC_FRAME_SAMPLES: usize = 1024;
/// Movie timescale for chapter times (milliseconds).
const CHAPTER_TIMESCALE: u64 = 1000;

/// Cover image container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverFormat {
    /// JPEG (`data` type 13).
    Jpeg,
    /// PNG (`data` type 14).
    Png,
}

impl CoverFormat {
    fn data_type(self) -> u32 {
        match self {
            Self::Jpeg => 13,
            Self::Png => 14,
        }
    }
}

/// Cover artwork embedded in the M4B `covr` atom.
#[derive(Debug, Clone, PartialEq)]
pub struct M4bCover {
    /// Image encoding.
    pub format: CoverFormat,
    /// Raw image bytes.
    pub data: Vec<u8>,
}

/// Audiobook metadata written to the `ilst` atom.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct M4bMetadata {
    /// Book title (`©nam`).
    pub title: String,
    /// Narrator, mapped to `©ART` so readers display it as the artist.
    pub narrator: String,
    /// Series name, mapped to `©alb`.
    pub series: String,
    /// Position in the series (`tvsn`, numeric).
    pub series_number: Option<u32>,
    /// Free-text description (`©cmt`).
    pub comment: String,
}

/// One chapter marker written to the `chpl` atom.
#[derive(Debug, Clone, PartialEq)]
pub struct M4bChapter {
    /// Chapter title.
    pub title: String,
    /// Start time in seconds.
    pub start_secs: f64,
    /// Duration in seconds.
    pub duration_secs: f64,
}

/// Result of an M4B export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct M4bExportReport {
    /// Destination path.
    pub path: String,
    /// AAC frames written.
    pub frames: usize,
    /// Audio duration in seconds.
    pub duration_secs: f64,
    /// Output bitrate in bits per second.
    pub bitrate: u32,
    /// Chapter count embedded in `chpl`.
    pub chapter_count: usize,
    /// Whether cover art was embedded.
    pub has_cover: bool,
}

fn aac_sample_rate(sample_rate: u32) -> bool {
    matches!(
        sample_rate,
        96_000
            | 88_200
            | 64_000
            | 48_000
            | 44_100
            | 32_000
            | 24_000
            | 22_050
            | 16_000
            | 12_000
            | 11_025
            | 8_000
    )
}

fn freq_index(rate: u32) -> SampleFreqIndex {
    match rate {
        96_000 => SampleFreqIndex::Freq96000,
        88_200 => SampleFreqIndex::Freq88200,
        64_000 => SampleFreqIndex::Freq64000,
        44_100 => SampleFreqIndex::Freq44100,
        32_000 => SampleFreqIndex::Freq32000,
        24_000 => SampleFreqIndex::Freq24000,
        22_050 => SampleFreqIndex::Freq22050,
        16_000 => SampleFreqIndex::Freq16000,
        12_000 => SampleFreqIndex::Freq12000,
        11_025 => SampleFreqIndex::Freq11025,
        8_000 => SampleFreqIndex::Freq8000,
        _ => SampleFreqIndex::Freq48000,
    }
}

fn box_bytes(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let size = (8 + payload.len()) as u32;
    let mut out = Vec::with_capacity(size as usize);
    out.extend_from_slice(&size.to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    out
}

fn data_box(data_type: u32, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::with_capacity(8 + payload.len());
    body.extend_from_slice(&data_type.to_be_bytes());
    body.extend_from_slice(&0u32.to_be_bytes()); // locale
    body.extend_from_slice(payload);
    box_bytes(b"data", &body)
}

fn text_tag(kind: &[u8; 4], value: &str) -> Vec<u8> {
    if value.is_empty() {
        return Vec::new();
    }
    box_bytes(kind, &data_box(1, value.as_bytes()))
}

fn number_tag(kind: &[u8; 4], value: u32) -> Vec<u8> {
    box_bytes(kind, &data_box(21, &value.to_be_bytes()))
}

fn chapters_box(chapters: &[M4bChapter]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.push(1); // version
    payload.extend_from_slice(&[0, 0, 0]); // flags
    payload.extend_from_slice(&(chapters.len() as u32).to_be_bytes());
    for chapter in chapters {
        let start = (chapter.start_secs * CHAPTER_TIMESCALE as f64)
            .round()
            .max(0.0) as u64;
        let duration = (chapter.duration_secs * CHAPTER_TIMESCALE as f64)
            .round()
            .max(0.0) as u64;
        payload.extend_from_slice(&start.to_be_bytes());
        payload.extend_from_slice(&duration.to_be_bytes());
        let title = chapter.title.as_bytes();
        let len = title.len().min(u16::MAX as usize) as u16;
        payload.extend_from_slice(&len.to_be_bytes());
        payload.extend_from_slice(&title[..len as usize]);
    }
    box_bytes(b"chpl", &payload)
}

fn cover_box(cover: &M4bCover) -> Vec<u8> {
    if cover.data.is_empty() {
        return Vec::new();
    }
    let data = data_box(cover.format.data_type(), &cover.data);
    box_bytes(b"covr", &data)
}

fn metadata_udta(
    chapters: &[M4bChapter],
    cover: Option<&M4bCover>,
    metadata: &M4bMetadata,
) -> Vec<u8> {
    let mut udta = chapters_box(chapters);
    if let Some(cover) = cover {
        udta.extend_from_slice(&cover_box(cover));
    }

    let mut ilst = Vec::new();
    ilst.extend_from_slice(&text_tag(b"\xa9nam", &metadata.title));
    ilst.extend_from_slice(&text_tag(b"\xa9ART", &metadata.narrator));
    ilst.extend_from_slice(&text_tag(b"\xa9alb", &metadata.series));
    ilst.extend_from_slice(&text_tag(b"\xa9cmt", &metadata.comment));
    if let Some(number) = metadata.series_number {
        ilst.extend_from_slice(&number_tag(b"tvsn", number));
    }

    let mut hdlr = Vec::new();
    hdlr.extend_from_slice(&0u32.to_be_bytes()); // version + flags
    hdlr.extend_from_slice(&0u32.to_be_bytes()); // pre_defined
    hdlr.extend_from_slice(b"mdir");
    hdlr.extend_from_slice(&[0u8; 12]); // reserved
    hdlr.extend_from_slice(b"ALP Metadata\0");

    let mut meta_payload = Vec::new();
    meta_payload.extend_from_slice(&0u32.to_be_bytes()); // version + flags
    meta_payload.extend_from_slice(&box_bytes(b"hdlr", &hdlr));
    meta_payload.extend_from_slice(&box_bytes(b"ilst", &ilst));
    udta.extend_from_slice(&box_bytes(b"meta", &meta_payload));

    box_bytes(b"udta", &udta)
}

/// Insert a `udta` box at the end of the top-level `moov` box.
fn inject_udta(file: &mut Vec<u8>, udta: &[u8]) -> Result<(), MediaError> {
    let mut position = 0usize;
    while position + 8 <= file.len() {
        let size = u32::from_be_bytes(
            file[position..position + 4]
                .try_into()
                .map_err(|_| MediaError::Encode("invalid box header".into()))?,
        ) as usize;
        let kind = &file[position + 4..position + 8];
        if size < 8 {
            return Err(MediaError::Encode("invalid box size in M4B".into()));
        }
        let end = position.saturating_add(size).min(file.len());
        if kind == b"moov" {
            let new_size = size + udta.len();
            if new_size > u32::MAX as usize {
                return Err(MediaError::Encode("M4B moov box too large".into()));
            }
            file.splice(end..end, udta.iter().copied());
            file[position..position + 4].copy_from_slice(&(new_size as u32).to_be_bytes());
            return Ok(());
        }
        position = end;
    }
    Err(MediaError::Encode("M4B writer produced no moov box".into()))
}

/// Encode PCM as an AAC-LC M4B audiobook.
///
/// `settings.sample_rate` is honoured when it is an AAC-supported rate; other
/// rates are resampled to 48 kHz before encoding.
pub fn export_m4b(
    path: impl AsRef<Path>,
    buffer: &PcmBuffer,
    settings: &crate::export::ExportSettings,
    chapters: &[M4bChapter],
    cover: Option<&M4bCover>,
    metadata: &M4bMetadata,
) -> Result<M4bExportReport, MediaError> {
    let requested_rate = settings.sample_rate.unwrap_or(buffer.sample_rate());
    let prepared = if aac_sample_rate(requested_rate) {
        std::borrow::Cow::Borrowed(buffer)
    } else {
        std::borrow::Cow::Owned(resample(buffer, 48_000)?)
    };
    let sample_rate = prepared.sample_rate();
    let channels = prepared.channels();
    if !(1..=2).contains(&channels) {
        return Err(MediaError::Encode(format!(
            "M4B supports mono or stereo input, got {channels} channels"
        )));
    }
    let bitrate = if channels == 1 { 64_000 } else { 128_000 };

    let encoder_config = EncoderConfig {
        sample_rate,
        channels: channels as u8,
        bitrate,
    };
    let mut encoder = StreamEncoder::new(encoder_config)
        .map_err(|error| MediaError::Encode(format!("AAC encoder setup failed: {error:?}")))?;

    let config = Mp4Config {
        major_brand: "isom"
            .parse()
            .map_err(|e| MediaError::Encode(format!("{e:?}")))?,
        minor_version: 512,
        compatible_brands: vec![
            "isom"
                .parse()
                .map_err(|e| MediaError::Encode(format!("{e:?}")))?,
            "iso2"
                .parse()
                .map_err(|e| MediaError::Encode(format!("{e:?}")))?,
            "mp41"
                .parse()
                .map_err(|e| MediaError::Encode(format!("{e:?}")))?,
            "M4B "
                .parse()
                .map_err(|e| MediaError::Encode(format!("{e:?}")))?,
        ],
        timescale: sample_rate,
    };
    let aac = AacConfig {
        bitrate,
        profile: AudioObjectType::AacLowComplexity,
        freq_index: freq_index(sample_rate),
        chan_conf: if channels == 1 {
            ChannelConfig::Mono
        } else {
            ChannelConfig::Stereo
        },
    };
    let track = TrackConfig {
        track_type: mp4::TrackType::Audio,
        timescale: sample_rate,
        language: "und".to_owned(),
        media_conf: MediaConfig::AacConfig(aac),
    };

    let mut writer = Mp4Writer::write_start(Cursor::new(Vec::new()), &config)
        .map_err(|error| MediaError::Encode(format!("M4B write start failed: {error:?}")))?;
    writer
        .add_track(&track)
        .map_err(|error| MediaError::Encode(format!("M4B track setup failed: {error:?}")))?;

    let samples = prepared.samples();
    let pcm: Vec<i16> = samples
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16)
        .collect();
    let frame_len = AAC_FRAME_SAMPLES * usize::from(channels);
    let mut frames = 0usize;
    for chunk in pcm.chunks(frame_len) {
        let adts = encoder
            .encode_frame(chunk)
            .map_err(|error| MediaError::Encode(format!("AAC encode failed: {error:?}")))?;
        let (_, payload_offset) = AdtsHeader::parse(&adts)
            .map_err(|error| MediaError::Encode(format!("invalid ADTS frame: {error:?}")))?;
        let sample = Mp4Sample {
            start_time: (frames * AAC_FRAME_SAMPLES) as u64,
            duration: AAC_FRAME_SAMPLES as u32,
            rendering_offset: 0,
            is_sync: true,
            bytes: Bytes::copy_from_slice(&adts[payload_offset..]),
        };
        writer
            .write_sample(1, &sample)
            .map_err(|error| MediaError::Encode(format!("M4B sample write failed: {error:?}")))?;
        frames += 1;
    }
    writer
        .write_end()
        .map_err(|error| MediaError::Encode(format!("M4B finalise failed: {error:?}")))?;
    let mut file = writer.into_writer().into_inner();
    inject_udta(&mut file, &metadata_udta(chapters, cover, metadata))?;
    write_atomic(path.as_ref(), &file)?;

    Ok(M4bExportReport {
        path: path.as_ref().display().to_string(),
        frames,
        duration_secs: prepared.duration_secs(),
        bitrate,
        chapter_count: chapters.len(),
        has_cover: cover.is_some_and(|cover| !cover.data.is_empty()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::export::ExportSettings;
    use std::f32::consts::PI;

    #[test]
    fn exports_m4b_with_chapters_cover_and_tags() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("book.m4b");
        let samples: Vec<f32> = (0..48_000 / 2)
            .map(|i| 0.2 * (2.0 * PI * 440.0 * i as f32 / 48_000.0).sin())
            .collect();
        let buffer = PcmBuffer::new(48_000, 1, samples).expect("pcm");
        let chapters = [
            M4bChapter {
                title: "Opening".into(),
                start_secs: 0.0,
                duration_secs: 0.25,
            },
            M4bChapter {
                title: "Second".into(),
                start_secs: 0.25,
                duration_secs: 0.25,
            },
        ];
        let cover = M4bCover {
            format: CoverFormat::Jpeg,
            data: vec![0xFF, 0xD8, 0xFF, 0xD9],
        };
        let metadata = M4bMetadata {
            title: "Test Book".into(),
            narrator: "Marcus Reed".into(),
            series: "Audio Labs Tests".into(),
            series_number: Some(1),
            comment: "Generated fixture".into(),
        };

        let report = export_m4b(
            &target,
            &buffer,
            &ExportSettings::wav(crate::wav::WavBitDepth::Int16),
            &chapters,
            Some(&cover),
            &metadata,
        )
        .expect("export m4b");

        assert!(target.is_file());
        assert!(report.frames > 0);
        assert_eq!(report.chapter_count, 2);
        assert!(report.has_cover);
        let bytes = std::fs::read(&target).expect("read m4b");
        assert!(bytes.windows(4).any(|w| w == b"chpl"));
        assert!(bytes.windows(4).any(|w| w == b"covr"));
        assert!(bytes.windows(4).any(|w| w == b"ilst"));
    }
}
