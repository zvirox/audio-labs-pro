//! `alp-media` — decoding, resampling, peak pyramids and WAV output.
//!
//! Phase 1 supports WAV, FLAC, MP3, OGG Vorbis and AIFF decoding through
//! symphonia, offline resampling through rubato, and 16/24-bit or float WAV
//! output through hound. See `docs/modules/alp-media.md`.

mod cache;
mod decode;
mod error;
mod pcm;
mod peaks;
mod resample;
mod synthetic;
mod wav;

pub use cache::{CacheStatus, PeakCache};
pub use decode::{MediaInfo, decode, decode_path, probe};
pub use error::MediaError;
pub use pcm::PcmBuffer;
pub use peaks::{
    BASE_BUCKET, CACHE_MAGIC, CACHE_VERSION, LEVEL_COUNT, PeakBuilder, PeakLevel, PeakPyramid,
};
pub use resample::resample;
pub use synthetic::write_test_tone;
pub use wav::{WavBitDepth, write_wav};

/// Peak cache directory name inside a project.
pub const PEAKS_DIR: &str = "peaks";
