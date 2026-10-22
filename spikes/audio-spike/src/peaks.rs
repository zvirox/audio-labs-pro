//! Decode with symphonia and build a min/max peak pyramid, then cache it to
//! disk. Timings here feed the Phase 1/2 performance budget.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::time::Instant;

use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;

const BASE_BUCKET: usize = 256;
const LEVELS: usize = 3;
const MAGIC: &[u8; 8] = b"ALPPEAKS";

struct Level {
    /// Interleaved i16 min/max pairs.
    pairs: Vec<(i16, i16)>,
    bucket_samples: usize,
}

pub fn run(input: &str, cache: &str) -> Result<(), String> {
    let started = Instant::now();

    let file = File::open(input).map_err(|e| format!("open {input}: {e}"))?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(extension) = std::path::Path::new(input)
        .extension()
        .and_then(|e| e.to_str())
    {
        hint.with_extension(extension);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("probe: {e}"))?;

    let track = format
        .default_track(TrackType::Audio)
        .ok_or("no audio track")?;
    let track_id = track.id;
    let codec_params = track
        .codec_params
        .as_ref()
        .ok_or("missing codec parameters")?
        .audio()
        .ok_or("not an audio codec")?
        .clone();

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&codec_params, &AudioDecoderOptions::default())
        .map_err(|e| format!("decoder: {e}"))?;

    let mut levels = Vec::with_capacity(LEVELS);
    let mut base: Vec<(i16, i16)> = Vec::new();
    let mut min = i16::MAX;
    let mut max = i16::MIN;
    let mut frame_in_bucket = 0usize;
    let mut total_frames = 0u64;
    let mut sample_rate = codec_params.sample_rate.unwrap_or(48_000);
    let mut scratch: Vec<f32> = Vec::new();
    let mut packets = 0u64;

    loop {
        let packet = match format.next_packet() {
            Ok(Some(packet)) => packet,
            Ok(None) => break,
            Err(SymphoniaError::ResetRequired) => break,
            Err(error) => return Err(format!("next_packet: {error}")),
        };

        if packet.track_id != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(SymphoniaError::IoError(_)) | Err(SymphoniaError::DecodeError(_)) => continue,
            Err(error) => return Err(format!("decode: {error}")),
        };

        packets += 1;
        sample_rate = decoded.spec().rate();

        decay_scratch(
            &mut scratch,
            decoded.frames() * decoded.spec().channels().count(),
        );
        decoded.copy_to_vec_interleaved(&mut scratch);

        for chunk in scratch.chunks(decoded.spec().channels().count()) {
            let value = chunk.iter().copied().sum::<f32>() / chunk.len() as f32;
            let sample = (value.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            min = min.min(sample);
            max = max.max(sample);
            frame_in_bucket += 1;

            if frame_in_bucket == BASE_BUCKET {
                base.push((min, max));
                min = i16::MAX;
                max = i16::MIN;
                frame_in_bucket = 0;
            }
        }

        total_frames += decoded.frames() as u64;
    }

    if frame_in_bucket > 0 {
        base.push((min, max));
    }

    if base.is_empty() {
        return Err("decoded no audio frames".into());
    }

    let decoded_at = started.elapsed();
    levels.push(Level {
        pairs: base,
        bucket_samples: BASE_BUCKET,
    });

    for _ in 1..LEVELS {
        let previous = &levels.last().unwrap().pairs;
        let mut next = Vec::with_capacity(previous.len() / 4 + 1);
        for chunk in previous.chunks(4) {
            let lo = chunk.iter().map(|p| p.0).min().unwrap_or(0);
            let hi = chunk.iter().map(|p| p.1).max().unwrap_or(0);
            next.push((lo, hi));
        }
        let bucket_samples = levels.last().unwrap().bucket_samples * 4;
        levels.push(Level {
            pairs: next,
            bucket_samples,
        });
    }

    let pyramid_at = started.elapsed();
    println!(
        "decoded {total_frames} frames ({:.1}s @ {sample_rate} Hz) from {packets} packets in {:?}",
        total_frames as f64 / sample_rate as f64,
        decoded_at
    );
    for level in &levels {
        println!(
            "  level bucket={} buckets={}",
            level.bucket_samples,
            level.pairs.len()
        );
    }

    let bytes = write_cache(cache, sample_rate, total_frames, &levels)?;
    let wrote_at = started.elapsed();
    println!(
        "cache written: {bytes} bytes ({:.2} MB) in {:?}",
        bytes as f64 / 1_048_576.0,
        wrote_at - pyramid_at
    );

    let reload_started = Instant::now();
    let (rate, frames, loaded) = read_cache(cache)?;
    let reloaded_at = reload_started.elapsed();
    println!(
        "cache reloaded: {rate} Hz · {frames} frames · {} buckets in {:?}",
        loaded.iter().map(|l| l.2).sum::<usize>(),
        reloaded_at
    );

    println!("TOTAL decode+peaks+cache+reload: {:?}", started.elapsed());
    Ok(())
}

fn decay_scratch(scratch: &mut Vec<f32>, capacity: usize) {
    scratch.clear();
    if scratch.capacity() < capacity {
        scratch.reserve(capacity - scratch.capacity());
    }
}

fn write_cache(
    path: &str,
    sample_rate: u32,
    frames: u64,
    levels: &[Level],
) -> Result<usize, String> {
    let file = File::create(path).map_err(|e| format!("create cache: {e}"))?;
    let mut writer = BufWriter::new(file);

    let mut written = 0usize;
    writer.write_all(MAGIC).map_err(|e| e.to_string())?;
    writer
        .write_all(&1u32.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&sample_rate.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&frames.to_le_bytes())
        .map_err(|e| e.to_string())?;
    writer
        .write_all(&(levels.len() as u32).to_le_bytes())
        .map_err(|e| e.to_string())?;
    written += 8 + 4 + 4 + 8 + 4;

    for level in levels {
        writer
            .write_all(&(level.bucket_samples as u32).to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&(level.pairs.len() as u64).to_le_bytes())
            .map_err(|e| e.to_string())?;
        written += 4 + 8;

        let mut bytes = Vec::with_capacity(level.pairs.len() * 4);
        for (min, max) in &level.pairs {
            bytes.extend_from_slice(&min.to_le_bytes());
            bytes.extend_from_slice(&max.to_le_bytes());
        }
        writer.write_all(&bytes).map_err(|e| e.to_string())?;
        written += bytes.len();
    }

    writer.flush().map_err(|e| e.to_string())?;
    Ok(written)
}

type CachedLevels = Vec<(usize, usize, usize)>;

fn read_cache(path: &str) -> Result<(u32, u64, CachedLevels), String> {
    let file = File::open(path).map_err(|e| format!("open cache: {e}"))?;
    let mut reader = BufReader::new(file);

    let mut magic = [0u8; 8];
    reader.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic != MAGIC {
        return Err("bad cache magic".into());
    }

    let read_u32 = |reader: &mut BufReader<File>| -> Result<u32, String> {
        let mut buffer = [0u8; 4];
        reader.read_exact(&mut buffer).map_err(|e| e.to_string())?;
        Ok(u32::from_le_bytes(buffer))
    };

    let version = read_u32(&mut reader)?;
    if version != 1 {
        return Err(format!("unsupported cache version {version}"));
    }
    let sample_rate = read_u32(&mut reader)?;

    let mut frames_bytes = [0u8; 8];
    reader
        .read_exact(&mut frames_bytes)
        .map_err(|e| e.to_string())?;
    let frames = u64::from_le_bytes(frames_bytes);

    let level_count = read_u32(&mut reader)?;
    let mut levels = Vec::new();
    for _ in 0..level_count {
        let bucket_samples = read_u32(&mut reader)? as usize;
        let mut count_bytes = [0u8; 8];
        reader
            .read_exact(&mut count_bytes)
            .map_err(|e| e.to_string())?;
        let count = u64::from_le_bytes(count_bytes) as usize;
        let mut bytes = vec![0u8; count * 4];
        reader.read_exact(&mut bytes).map_err(|e| e.to_string())?;
        levels.push((bucket_samples, count, count));
    }

    Ok((sample_rate, frames, levels))
}
