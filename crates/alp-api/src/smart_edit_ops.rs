//! Smart editing operations (trim silence, shorten pauses, remove fillers,
//! loudness match) for `ApiSession`.
//!
//! These are real edits, not reports: each one measures the track, cuts the
//! timeline with ripple-delete commands (so undo works), and re-renders to
//! report what actually changed. When a measurement cannot support an edit —
//! most importantly filler removal without word-level timestamps — the call
//! fails with an explanation instead of reporting fabricated success.

use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::session::ApiSession;
use crate::types::PermissionTier;
use alp_core::{DeleteRange, FrameRange, SetTrackGain, TrackId, secs_to_frames};

/// Longest run of frames this module will cut in one command, so a single
/// agent call cannot turn into an unbounded command queue.
const MAX_CUT_RANGES: usize = 512;

/// Result of trimming silence from a track.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceTrimReport {
    /// How many seconds of silence were actually removed.
    pub silence_trimmed_secs: f64,
    /// Number of silence ranges cut from the track.
    pub ranges_cut: usize,
    /// Track length before and after the edit.
    pub before_secs: f64,
    pub after_secs: f64,
    /// Threshold in dBFS that defined silence.
    pub threshold_dbfs: f32,
    /// Minimum run length in seconds that was cut.
    pub min_silence_secs: f64,
}

/// Shorten pauses in a track (remove or compress pauses below a threshold).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseShortenReport {
    /// Pauses identified and shortened.
    pub pauses_processed: usize,
    /// Total seconds removed from the track.
    pub seconds_removed: f64,
    /// Track length before and after the edit.
    pub before_secs: f64,
    pub after_secs: f64,
    /// Pause length allowed to remain, in seconds.
    pub max_pause_secs: f64,
}

/// Remove filler words from a track based on transcript word timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillerRemoveReport {
    /// Filler words removed.
    pub fillers_removed: usize,
    /// Filler words that were detected.
    pub fillers_found: Vec<String>,
    /// Track length before and after the edit.
    pub before_secs: f64,
    pub after_secs: f64,
    /// Engine that produced the word timestamps.
    pub engine: String,
}

/// Match project loudness to a target LUFS value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoudnessMatchReport {
    /// Target LUFS that was requested.
    pub target_lufs: f32,
    /// Measured LUFS before the edit.
    pub before_lufs: f32,
    /// Measured LUFS after the edit.
    pub actual_lufs: f32,
    /// Sample peak after the edit in dBFS.
    pub peak_dbfs: f32,
    /// Gain actually applied in dB.
    pub gain_db: f32,
    /// True when the gain was reduced to keep the peak below the ceiling.
    pub limited_by_peak: bool,
}

/// A silent run of frames on the timeline.
#[derive(Debug, Clone, Copy)]
struct SilentRun {
    start: u64,
    end: u64,
}

impl ApiSession {
    /// Cut frame ranges out of a track, latest first so earlier ranges keep
    /// their timeline positions. Returns the number of commands applied.
    fn cut_track_ranges(
        &mut self,
        track: TrackId,
        ranges: &[FrameRange],
    ) -> Result<usize, ApiError> {
        let mut ordered: Vec<FrameRange> =
            ranges.iter().copied().filter(|r| !r.is_empty()).collect();
        ordered.sort_by_key(|range| range.start);
        ordered.dedup();
        if ordered.len() > MAX_CUT_RANGES {
            return Err(ApiError::Validation(format!(
                "refusing to cut {} ranges in one call (limit {MAX_CUT_RANGES}); widen the thresholds or clean the track in passes",
                ordered.len()
            )));
        }
        let applied = ordered.len();
        for range in ordered.iter().rev() {
            self.execute_command(
                "range_edit",
                Box::new(DeleteRange::new(track, *range, true)),
            )?;
        }
        Ok(applied)
    }

    /// Detect silent runs on a rendered track buffer. A run is silent when the
    /// per-frame peak across channels stays under `threshold_db`.
    fn silent_runs(
        samples: &[f32],
        channels: u16,
        threshold_db: f32,
        min_frames: u64,
    ) -> Vec<SilentRun> {
        let channels = usize::from(channels.max(1));
        let threshold = 10.0f32.powf(threshold_db / 20.0);
        let frames = samples.len() / channels;
        let mut runs = Vec::new();
        let mut run_start: Option<u64> = None;
        for frame in 0..frames {
            let peak = (0..channels)
                .map(|c| samples[frame * channels + c].abs())
                .fold(0.0f32, f32::max);
            match (peak < threshold, run_start) {
                (true, None) => run_start = Some(frame as u64),
                (false, Some(start)) => {
                    if frame as u64 - start >= min_frames {
                        runs.push(SilentRun {
                            start,
                            end: frame as u64,
                        });
                    }
                    run_start = None;
                }
                _ => {}
            }
        }
        if let Some(start) = run_start
            && frames as u64 - start >= min_frames
        {
            runs.push(SilentRun {
                start,
                end: frames as u64,
            });
        }
        runs
    }

    /// Trim silence from a track below a dBFS threshold. Leading, interior and
    /// trailing silence are all cut, and the track ripples closed.
    pub fn edit_trim_silence(
        &mut self,
        track_id: &str,
        threshold_db: f32,
        min_silence_secs: f64,
    ) -> Result<SilenceTrimReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        if !threshold_db.is_finite() || !(-96.0..=0.0).contains(&threshold_db) {
            return Err(ApiError::Validation(format!(
                "invalid threshold {threshold_db} dBFS: use a value between -96 and 0"
            )));
        }
        if !min_silence_secs.is_finite() || min_silence_secs < 0.0 {
            return Err(ApiError::Validation(format!(
                "invalid min_silence_secs {min_silence_secs}: must be zero or positive"
            )));
        }

        let rate = self.project.sample_rate;
        let (track, buffer) = self.render_track_mixdown(track_id)?;
        let before_secs = buffer.duration_secs();
        let min_frames = secs_to_frames(min_silence_secs, rate);

        let runs = Self::silent_runs(
            buffer.samples(),
            buffer.channels(),
            threshold_db,
            min_frames,
        );
        if runs.is_empty() {
            return Ok(SilenceTrimReport {
                silence_trimmed_secs: 0.0,
                ranges_cut: 0,
                before_secs,
                after_secs: before_secs,
                threshold_dbfs: threshold_db,
                min_silence_secs,
            });
        }

        let ranges: Vec<FrameRange> = runs
            .iter()
            .filter_map(|run| FrameRange::new(run.start, run.end).ok())
            .collect();
        let ranges_cut = self.cut_track_ranges(track, &ranges)?;

        let after_secs = self.render_track_mixdown(track_id)?.1.duration_secs();
        Ok(SilenceTrimReport {
            silence_trimmed_secs: (before_secs - after_secs).max(0.0),
            ranges_cut,
            before_secs,
            after_secs,
            threshold_dbfs: threshold_db,
            min_silence_secs,
        })
    }

    /// Shorten pauses longer than `max_pause_secs`, keeping the pause and
    /// cutting only the excess. The cut is taken from the end of each pause so
    /// the following word keeps its onset.
    pub fn edit_shorten_pauses(
        &mut self,
        track_id: &str,
        max_pause_secs: f64,
        crossfade_ms: u64,
    ) -> Result<PauseShortenReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        if !max_pause_secs.is_finite() || max_pause_secs < 0.0 {
            return Err(ApiError::Validation(format!(
                "invalid max_pause_secs {max_pause_secs}: must be zero or positive"
            )));
        }
        let _ = crossfade_ms; // crossfades are applied by the ripple command

        let rate = self.project.sample_rate;
        let (track, buffer) = self.render_track_mixdown(track_id)?;
        let before_secs = buffer.duration_secs();
        let threshold_db = -45.0f32;
        let min_frames = secs_to_frames(0.1, rate);
        let keep_frames = secs_to_frames(max_pause_secs, rate);

        let runs = Self::silent_runs(
            buffer.samples(),
            buffer.channels(),
            threshold_db,
            min_frames,
        );
        let mut ranges = Vec::new();
        for run in &runs {
            let length = run.end - run.start;
            if length > keep_frames {
                let excess = length - keep_frames;
                // Keep the head of the pause, cut the tail before the next word.
                ranges.push(
                    FrameRange::new(run.end - excess, run.end)
                        .map_err(|e| ApiError::Validation(format!("invalid pause range: {e}")))?,
                );
            }
        }

        if ranges.is_empty() {
            return Ok(PauseShortenReport {
                pauses_processed: 0,
                seconds_removed: 0.0,
                before_secs,
                after_secs: before_secs,
                max_pause_secs,
            });
        }

        self.cut_track_ranges(track, &ranges)?;
        let after_secs = self.render_track_mixdown(track_id)?.1.duration_secs();
        Ok(PauseShortenReport {
            pauses_processed: ranges.len(),
            seconds_removed: (before_secs - after_secs).max(0.0),
            before_secs,
            after_secs,
            max_pause_secs,
        })
    }

    /// Remove filler words from a track using transcript word timestamps.
    ///
    /// Filler removal depends on word-level alignment. The built-in segmenter
    /// emits placeholder words with no timing, so this refuses rather than
    /// cutting audio at invented positions.
    pub fn edit_remove_fillers(
        &mut self,
        track_id: &str,
        filler_word_ids: Option<Vec<String>>,
    ) -> Result<FillerRemoveReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;

        let default_fillers = ["um", "uh", "er", "uhm", "hmm", "mmm"];
        let targets: Vec<String> = filler_word_ids
            .filter(|list| !list.is_empty())
            .unwrap_or_else(|| default_fillers.iter().map(|s| (*s).to_owned()).collect())
            .into_iter()
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        if targets.is_empty() {
            return Err(ApiError::Validation(
                "no filler words to remove: pass filler_word_ids or omit the argument for the default list"
                    .into(),
            ));
        }

        let (track, buffer) = self.render_track_mixdown(track_id)?;
        let before_secs = buffer.duration_secs();

        let mut engine = String::from("builtin");
        let segments = crate::voice::transcribe_daemon_first(&buffer, &mut engine)?;
        if engine == "builtin" {
            return Err(ApiError::Validation(
                "filler removal needs word-level timestamps and the built-in segmenter only emits placeholder words; start alp-infer with a speech model (see transcribe) and retry".into(),
            ));
        }

        let mut ranges = Vec::new();
        let mut found = Vec::new();
        for segment in &segments {
            for word in &segment.words {
                let clean = word
                    .word
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_lowercase();
                if clean.is_empty() || !targets.contains(&clean) {
                    continue;
                }
                if let Ok(range) = FrameRange::new(word.start_frame, word.end_frame) {
                    found.push(clean);
                    ranges.push(range);
                }
            }
        }

        if ranges.is_empty() {
            return Ok(FillerRemoveReport {
                fillers_removed: 0,
                fillers_found: Vec::new(),
                before_secs,
                after_secs: before_secs,
                engine,
            });
        }

        self.cut_track_ranges(track, &ranges)?;
        let after_secs = self.render_track_mixdown(track_id)?.1.duration_secs();
        Ok(FillerRemoveReport {
            fillers_removed: ranges.len(),
            fillers_found: found,
            before_secs,
            after_secs,
            engine,
        })
    }

    /// Match project loudness to a target LUFS value by scaling every track's
    /// gain, then re-measuring the render. If the requested gain would push the
    /// sample peak above the ceiling, the gain is reduced and the report says
    /// so — the project is never silently clipped.
    pub fn edit_match_loudness(
        &mut self,
        target_lufs: f32,
    ) -> Result<LoudnessMatchReport, ApiError> {
        self.check_permission(PermissionTier::Edit)?;
        if !target_lufs.is_finite() || !(-70.0..=0.0).contains(&target_lufs) {
            return Err(ApiError::Validation(format!(
                "invalid target {target_lufs} LUFS: use a value between -70 and 0"
            )));
        }

        let peak_ceiling_db = -1.0f32;
        let buffer = self.render_project_mixdown()?;
        let before_lufs =
            alp_dsp::measure_lufs(buffer.samples(), buffer.sample_rate(), buffer.channels());
        if buffer.samples().iter().all(|s| s.abs() < 1e-6) {
            return Err(ApiError::Validation(
                "cannot match loudness: the project renders to digital silence".into(),
            ));
        }

        let tracks = self.track_gains();
        if tracks.is_empty() {
            return Err(ApiError::Validation(
                "cannot match loudness: the project has no tracks".into(),
            ));
        }
        let requested_gain_db = target_lufs - before_lufs;
        let peak_dbfs = 20.0 * buffer.peak().max(1e-6).log10();
        let headroom_db = peak_ceiling_db - peak_dbfs;
        let limited_by_peak = requested_gain_db > headroom_db;
        let gain_db = if limited_by_peak {
            headroom_db.max(0.0)
        } else {
            requested_gain_db
        };
        let linear = 10.0f32.powf(gain_db / 20.0);

        for (track_id, current) in tracks {
            self.execute_command(
                "track_gain",
                Box::new(SetTrackGain::new(track_id, current * linear)),
            )?;
        }

        let after = self.render_project_mixdown()?;
        let actual_lufs =
            alp_dsp::measure_lufs(after.samples(), after.sample_rate(), after.channels());
        let after_peak = 20.0 * after.peak().max(1e-6).log10();

        Ok(LoudnessMatchReport {
            target_lufs,
            before_lufs,
            actual_lufs,
            peak_dbfs: after_peak,
            gain_db,
            limited_by_peak,
        })
    }

    /// Current linear gain of every track, paired with its id.
    fn track_gains(&self) -> Vec<(TrackId, f32)> {
        self.project
            .tracks
            .iter()
            .map(|track| (track.id, track.gain))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditLogger;
    use crate::sandbox::WorkspaceSandbox;
    use alp_core::{Clip, ClipId, Project, ProjectPaths, Source, SourceId, Track, TrackId};

    /// Write a WAV whose signal is `pattern` seconds long and returns a session
    /// holding one track whose clips cover it.
    fn session_with_pattern(
        dir: &std::path::Path,
        pattern: &[(&str, f64)],
    ) -> (ApiSession, String) {
        let proj_paths = ProjectPaths::new(dir);
        proj_paths.ensure_layout().expect("layout");
        let rate = 48_000u32;

        let mut samples: Vec<f32> = Vec::new();
        for (kind, secs) in pattern {
            let count = (secs * f64::from(rate)) as usize;
            match *kind {
                "tone" => samples.extend((0..count).map(|i| {
                    0.5 * (i as f32 * 220.0 * 2.0 * std::f32::consts::PI / rate as f32).sin()
                })),
                _ => samples.extend(std::iter::repeat_n(0.0f32, count)),
            }
        }
        let frames = samples.len() as u64;
        let pcm = alp_media::PcmBuffer::new(rate, 1, samples).expect("pcm");
        let media_path = proj_paths.media_dir().join("speech.wav");
        alp_media::export(
            &media_path,
            &pcm,
            &alp_media::ExportSettings {
                format: alp_media::ExportFormat::Wav,
                bit_depth: alp_media::WavBitDepth::Int16,
                sample_rate: None,
                dither: alp_media::DitherMode::None,
                metadata: alp_media::AudioMetadata::default(),
            },
        )
        .expect("write fixture wav");

        let mut project = Project::new("Speech").expect("project");
        project.sources.push(
            Source::new(
                SourceId::from_u128(1),
                "speech.wav",
                "media/speech.wav",
                rate,
                1,
                frames,
            )
            .expect("source"),
        );
        let mut track = Track::new(TrackId::from_u128(2), "VoiceTrack");
        track.clips.push(Clip::new(
            ClipId::from_u128(3),
            SourceId::from_u128(1),
            FrameRange::new(0, frames).expect("range"),
            0,
        ));
        let track_id = track.id.to_string();
        project.tracks.push(track);

        let sandbox = WorkspaceSandbox::new([dir]).expect("sandbox");
        let audit = AuditLogger::new(dir.join("audit.jsonl")).expect("audit");
        let session = ApiSession::new(
            project,
            proj_paths,
            PermissionTier::Edit,
            sandbox,
            audit,
            "test-agent",
        )
        .expect("session");
        (session, track_id)
    }

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn trim_silence_actually_cuts_and_reports_the_new_length() {
        let dir = tmp();
        let (mut session, track_id) = session_with_pattern(
            dir.path(),
            &[("silence", 0.5), ("tone", 1.0), ("silence", 0.5)],
        );

        let report = session
            .edit_trim_silence(&track_id, -45.0, 0.2)
            .expect("trim silence");

        assert_eq!(report.ranges_cut, 2, "leading and trailing silence are cut");
        assert!(
            (report.before_secs - 2.0).abs() < 0.05,
            "before {}",
            report.before_secs
        );
        assert!(
            (report.after_secs - 1.0).abs() < 0.1,
            "the track must actually shrink to the tone, got {}s",
            report.after_secs
        );
        assert!(report.silence_trimmed_secs > 0.8);
    }

    #[test]
    fn trim_silence_without_silence_is_a_no_op() {
        let dir = tmp();
        let (mut session, track_id) = session_with_pattern(dir.path(), &[("tone", 1.0)]);
        let report = session
            .edit_trim_silence(&track_id, -45.0, 0.1)
            .expect("trim silence");
        assert_eq!(report.ranges_cut, 0);
        assert!((report.before_secs - report.after_secs).abs() < 1e-9);
    }

    #[test]
    fn shorten_pauses_keeps_the_allowed_pause_and_cuts_the_excess() {
        let dir = tmp();
        let (mut session, track_id) = session_with_pattern(
            dir.path(),
            &[("tone", 0.5), ("silence", 1.5), ("tone", 0.5)],
        );

        let report = session
            .edit_shorten_pauses(&track_id, 0.3, 10)
            .expect("shorten pauses");

        assert_eq!(report.pauses_processed, 1);
        assert!(
            (report.after_secs - 1.3).abs() < 0.15,
            "1.5s pause shortened to 0.3s leaves ~1.3s, got {}s",
            report.after_secs
        );
        assert!(report.seconds_removed > 1.0);
    }

    #[test]
    fn filler_removal_refuses_when_only_placeholder_words_exist() {
        let dir = tmp();
        let (mut session, track_id) = session_with_pattern(dir.path(), &[("tone", 1.0)]);

        let error = session
            .edit_remove_fillers(&track_id, Some(vec!["um".to_string()]))
            .expect_err("placeholder words must not drive cuts");
        assert!(
            error.to_string().contains("word-level timestamps"),
            "the refusal must explain the missing alignment, got {error}"
        );
    }

    #[test]
    fn match_loudness_applies_real_gain_and_respects_the_peak_ceiling() {
        let dir = tmp();
        let (mut session, _track_id) = session_with_pattern(dir.path(), &[("tone", 3.0)]);

        let before = session
            .analyze_loudness()
            .expect("loudness")
            .integrated_lufs;
        let report = session.edit_match_loudness(before + 3.0).expect("match");
        assert!(report.gain_db > 0.0, "gain {}", report.gain_db);
        assert!(
            (report.actual_lufs - report.before_lufs - report.gain_db).abs() < 0.5,
            "the applied gain must move the measurement, {} -> {} for {} dB",
            report.before_lufs,
            report.actual_lufs,
            report.gain_db
        );

        // Ask for a level that cannot be reached without clipping.
        let loud = session.edit_match_loudness(-1.0).expect("match");
        assert!(loud.limited_by_peak);
        assert!(
            loud.peak_dbfs <= -0.9,
            "peak {} must stay under the ceiling",
            loud.peak_dbfs
        );
    }
}
