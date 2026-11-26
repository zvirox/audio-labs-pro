//! `alp-media` — decoding, resampling, peak pyramids and WAV output.
//!
//! Phase 1 supports WAV, FLAC, MP3, OGG Vorbis and AIFF decoding through
//! symphonia, offline resampling through rubato, and 16/24-bit or float WAV
//! output through hound. See `docs/modules/alp-media.md`.

mod cache;
mod compare;
mod decode;
mod dither;
mod error;
mod export;
mod m4b;
mod pcm;
mod peaks;
mod resample;
mod synthetic;
mod time_stretch;
mod wav;

pub use cache::{CacheStatus, PeakCache};
pub use compare::{NullReport, compare};
pub use decode::{MediaInfo, decode, decode_path, probe};
pub use dither::DitherMode;
pub use error::MediaError;
pub use export::{AudioMetadata, ExportFormat, ExportPreset, ExportReport, ExportSettings, export};
pub use m4b::{CoverFormat, M4bChapter, M4bCover, M4bExportReport, M4bMetadata, export_m4b};
pub use pcm::PcmBuffer;
pub use peaks::{
    BASE_BUCKET, CACHE_MAGIC, CACHE_VERSION, LEVEL_COUNT, PeakBuilder, PeakLevel, PeakPyramid,
};
pub use resample::resample;
pub use synthetic::write_test_tone;
pub use time_stretch::time_stretch;
pub use wav::{WavBitDepth, write_wav};

/// Peak cache directory name inside a project.
pub const PEAKS_DIR: &str = "peaks";
