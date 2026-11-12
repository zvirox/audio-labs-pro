//! Acoustic clone tier: speech synthesis re-coloured by a measured voiceprint.
//!
//! This is the lightweight end of the cloning spectrum. It does not reproduce
//! a person's identity — it reproduces their *timbre*: pitch, intonation range
//! and the shape of their spectral envelope. The voiceprint comes from real
//! signal analysis of the reference recording
//! ([`analyze_voiceprint`](crate::voiceprint::analyze_voiceprint)), and the
//! synthesized harmonic stack is shaped by the measured per-band gains, so the
//! output genuinely tracks the reference instead of imitating it by ear.
//!
//! The engine is dependency-free and runs at 48 kHz. It is the tier that keeps
//! the application useful on a machine with no neural model installed, and it
//! is the tier the heavy neural clone backend (`qwen3-tts`) will replace.
//!
//! Synthesis model: a pulse train at the voiceprint's fundamental frequency
//! whose harmonics are weighted by a three-formant resonance envelope
//! multiplied by the voiceprint's measured band gains.

use alp_media::PcmBuffer;

use crate::profile::{VoiceParams, VoiceProfile};
use crate::text::TextNormalizer;
use crate::voiceprint::{VoicePrint, analyze_voiceprint};
use crate::{CloneBackend, TtsBackend, TtsRequest, VoiceError, VoiceInfo, VoiceKind};

/// Output sample rate of the acoustic tier.
pub const SAMPLE_RATE: u32 = 48_000;

/// Catalogue id of this backend.
pub const MODEL_ID: &str = "acoustic-clone";

/// Number of harmonics generated per voiced phoneme.
const HARMONICS: usize = 48;

/// Bandwidth of the first formant in Hz.
const F1_BANDWIDTH: f32 = 80.0;

/// Bandwidth of the second formant in Hz.
const F2_BANDWIDTH: f32 = 110.0;

/// Bandwidth of the third formant in Hz.
const F3_BANDWIDTH: f32 = 160.0;

/// Relative weight of the first formant in the combined envelope.
const F1_WEIGHT: f32 = 1.0;

/// Relative weight of the second formant in the combined envelope.
const F2_WEIGHT: f32 = 0.55;

/// Relative weight of the third formant in the combined envelope.
const F3_WEIGHT: f32 = 0.3;

/// The formant targets for one phoneme.
#[derive(Debug, Clone, Copy)]
struct Formants {
    /// First formant frequency in Hz.
    f1: f32,
    /// Second formant frequency in Hz.
    f2: f32,
    /// Third formant frequency in Hz.
    f3: f32,
    /// Whether the phoneme is voiced.
    voiced: bool,
}

/// The formant targets of the English vowels and a generic voiced consonant.
fn phoneme_formants(character: char) -> Formants {
    match character.to_ascii_lowercase() {
        'a' => Formants {
            f1: 800.0,
            f2: 1200.0,
            f3: 2500.0,
            voiced: true,
        },
        'e' => Formants {
            f1: 500.0,
            f2: 1800.0,
            f3: 2600.0,
            voiced: true,
        },
        'i' => Formants {
            f1: 300.0,
            f2: 2200.0,
            f3: 2900.0,
            voiced: true,
        },
        'o' => Formants {
            f1: 500.0,
            f2: 900.0,
            f3: 2400.0,
            voiced: true,
        },
        'u' => Formants {
            f1: 350.0,
            f2: 800.0,
            f3: 2300.0,
            voiced: true,
        },
        's' | 'f' | 't' | 'k' | 'p' => Formants {
            f1: 0.0,
            f2: 0.0,
            f3: 4500.0,
            voiced: false,
        },
        _ => Formants {
            f1: 450.0,
            f2: 1400.0,
            f3: 2500.0,
            voiced: true,
        },
    }
}

/// Gain of a single formant resonance at `freq_hz`.
fn resonance(freq_hz: f32, center: f32, bandwidth: f32) -> f32 {
    if center <= 0.0 {
        return 0.0;
    }
    let distance = (freq_hz - center) / bandwidth;
    1.0 / (1.0 + distance * distance).sqrt()
}

/// Combined formant envelope at `freq_hz`.
fn envelope(freq_hz: f32, formants: &Formants) -> f32 {
    let f1 = resonance(freq_hz, formants.f1, F1_BANDWIDTH) * F1_WEIGHT;
    let f2 = resonance(freq_hz, formants.f2, F2_BANDWIDTH) * F2_WEIGHT;
    let f3 = resonance(freq_hz, formants.f3, F3_BANDWIDTH) * F3_WEIGHT;
    f1 + f2 + f3
}

/// The acoustic clone engine.
#[derive(Debug, Clone)]
pub struct AcousticCloneBackend {
    sample_rate: u32,
}

impl AcousticCloneBackend {
    /// Create the engine at 48 kHz.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sample_rate: SAMPLE_RATE,
        }
    }

    /// Create the engine at an explicit sample rate.
    #[must_use]
    pub fn with_sample_rate(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// Synthesize `text` with a given voiceprint.
    ///
    /// This is the shared core of the built-in tier (a neutral voiceprint) and
    /// the clone tier (the reference's measured voiceprint).
    fn render(
        &self,
        text: &str,
        voiceprint: &VoicePrint,
        params: &VoiceParams,
    ) -> Result<PcmBuffer, VoiceError> {
        let normalized = TextNormalizer::normalize(text);
        if normalized.trim().is_empty() {
            return PcmBuffer::new(self.sample_rate, 1, vec![0.0; 100])
                .map_err(|e| VoiceError::Backend(e.to_string()));
        }

        let speed = params.speed.clamp(0.5, 2.5);
        let pitch_scale = 2.0_f32.powf(params.pitch / 12.0);
        let base_f0 = (voiceprint.f0_hz * pitch_scale).clamp(50.0, 400.0);
        let jitter = voiceprint.f0_spread * base_f0 * 0.12;

        let phoneme_frames = ((90.0 / speed) * 0.001 * self.sample_rate as f32) as usize;
        let words: Vec<&str> = normalized.split_whitespace().collect();
        let mut samples: Vec<f32> = Vec::new();

        // Lead-in silence.
        samples.resize((0.05 * self.sample_rate as f32) as usize, 0.0);

        // Harmonic amplitudes are recomputed per phoneme because the formant
        // envelope changes with the vowel.
        let mut phase = 0.0f32;
        let mut seed = 0x2545_F491_4F6C_DD1Du64;

        for word in words.iter() {
            for character in word.chars() {
                if !character.is_alphabetic() {
                    continue;
                }
                let formants = phoneme_formants(character);

                // Harmonic weights: formant envelope times the measured
                // spectral gains, with a glottal-like 1/k rolloff.
                let mut weights = [0.0f32; HARMONICS];
                for (index, weight) in weights.iter_mut().enumerate() {
                    let harmonic = index + 1;
                    let freq = base_f0 * harmonic as f32;
                    if freq > self.sample_rate as f32 / 2.0 {
                        break;
                    }
                    let formant_gain = envelope(freq, &formants);
                    let measured_gain = voiceprint.gain_at(freq);
                    *weight = formant_gain * measured_gain / harmonic as f32;
                }

                for frame_index in 0..phoneme_frames {
                    let progress = frame_index as f32 / phoneme_frames as f32;
                    let env = if progress < 0.15 {
                        progress / 0.15
                    } else if progress > 0.85 {
                        (1.0 - progress) / 0.15
                    } else {
                        1.0
                    };

                    let sample = if formants.voiced {
                        // A lightly jittered fundamental keeps the cloned
                        // voice from sounding like a pure tone.
                        seed = seed
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1_442_695_040_888_963_407);
                        let noise = ((seed >> 33) as f32 / (u32::MAX as f32)) - 0.5;
                        let f0 = base_f0 + jitter * noise;
                        phase += f0 / self.sample_rate as f32;
                        if phase >= 1.0 {
                            phase -= 1.0;
                        }

                        let mut sum = 0.0f32;
                        for (index, weight) in weights.iter().enumerate() {
                            if *weight <= 1e-6 {
                                continue;
                            }
                            let harmonic = index + 1;
                            let angle = 2.0 * std::f32::consts::PI * harmonic as f32 * phase;
                            sum += weight * angle.sin();
                        }
                        sum * env * 0.6
                    } else {
                        // Shaped noise for unvoiced consonants.
                        seed = seed
                            .wrapping_mul(6_364_136_223_846_793_005)
                            .wrapping_add(1_442_695_040_888_963_407);
                        let noise = ((seed >> 33) as f32 / (u32::MAX as f32)) * 2.0 - 1.0;
                        noise * env * 0.16
                    };

                    samples.push(sample);
                }
            }

            // Word boundary gap.
            let word_gap = ((0.06 / speed) * self.sample_rate as f32) as usize;
            samples.resize(samples.len() + word_gap, 0.0);
        }

        // Tail padding.
        samples.resize(
            samples.len() + (0.05 * self.sample_rate as f32) as usize,
            0.0,
        );

        PcmBuffer::new(self.sample_rate, 1, samples).map_err(|e| VoiceError::Backend(e.to_string()))
    }
}

impl Default for AcousticCloneBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl TtsBackend for AcousticCloneBackend {
    fn id(&self) -> &str {
        MODEL_ID
    }

    fn voices(&self) -> Vec<VoiceInfo> {
        vec![VoiceInfo {
            id: MODEL_ID.into(),
            name: "Acoustic Clone".into(),
            language: "en".into(),
            kind: VoiceKind::Cloned,
        }]
    }

    fn synthesize(&self, request: &TtsRequest) -> Result<PcmBuffer, VoiceError> {
        self.render(
            &request.text,
            &VoicePrint::default(),
            &VoiceParams {
                speed: request.speed,
                ..VoiceParams::default()
            },
        )
    }
}

impl CloneBackend for AcousticCloneBackend {
    fn create_profile(
        &self,
        reference: &PcmBuffer,
        transcript: Option<&str>,
    ) -> Result<VoiceProfile, VoiceError> {
        if reference.frames() == 0 {
            return Err(VoiceError::Backend(
                "reference audio is empty; nothing to clone from".into(),
            ));
        }

        let voiceprint = analyze_voiceprint(reference);
        if voiceprint.frames == 0 {
            return Err(VoiceError::Backend(
                "reference audio contains no voiced speech; record a clear spoken phrase".into(),
            ));
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        Ok(VoiceProfile {
            id: format!("acoustic-clone-{timestamp}"),
            name: "Acoustic Clone".into(),
            kind: VoiceKind::Cloned,
            backend: MODEL_ID.into(),
            reference_audio: None,
            transcript: transcript.map(str::to_string),
            cached_embedding: Some(voiceprint.to_embedding()),
            tags: vec!["cloned".into(), "acoustic".into(), "timbre".into()],
            default_params: VoiceParams::default(),
            created_at: timestamp,
            // The consent gate is human-only: a clone profile is never
            // persisted without an attested record (see `ProfileStore`).
            consent: None,
        })
    }

    fn synthesize_with(
        &self,
        profile: &VoiceProfile,
        request: &TtsRequest,
    ) -> Result<PcmBuffer, VoiceError> {
        let voiceprint = match profile.cached_embedding.as_deref() {
            Some(embedding) => VoicePrint::from_embedding(embedding),
            None => VoicePrint::default(),
        };
        let params = VoiceParams {
            speed: request.speed * profile.default_params.speed,
            pitch: profile.default_params.pitch,
            energy: profile.default_params.energy,
            emotion: profile.default_params.emotion.clone(),
        };
        self.render(&request.text, &voiceprint, &params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voiceprint::analyze_voiceprint;

    /// A sustained vowel used as a reference recording.
    fn reference(f0: f32, rate: u32) -> PcmBuffer {
        let frames = (rate as f32) as usize;
        let samples: Vec<f32> = (0..frames)
            .map(|index| {
                let t = index as f32 / rate as f32;
                0.4 * (2.0 * std::f32::consts::PI * f0 * t).sin()
            })
            .collect();
        PcmBuffer::new(rate, 1, samples).expect("pcm")
    }

    #[test]
    fn a_profile_is_created_from_a_reference_recording() {
        let backend = AcousticCloneBackend::new();
        let profile = backend
            .create_profile(&reference(150.0, 16_000), Some("reference phrase"))
            .expect("create profile");

        assert_eq!(profile.kind, VoiceKind::Cloned);
        assert_eq!(profile.backend, MODEL_ID);
        assert!(profile.cached_embedding.is_some());
        // The consent record is never fabricated by the engine.
        assert!(profile.consent.is_none());
    }

    #[test]
    fn an_empty_reference_is_refused() {
        let backend = AcousticCloneBackend::new();
        let empty = PcmBuffer::new(16_000, 1, Vec::new()).expect("pcm");
        let error = backend
            .create_profile(&empty, None)
            .expect_err("must refuse");
        assert!(error.to_string().contains("empty"), "{error}");
    }

    #[test]
    fn a_silent_reference_is_refused_with_guidance() {
        let backend = AcousticCloneBackend::new();
        let silence = PcmBuffer::new(16_000, 1, vec![0.0; 16_000]).expect("pcm");
        let error = backend
            .create_profile(&silence, None)
            .expect_err("must refuse");
        assert!(error.to_string().contains("voiced speech"), "{error}");
    }

    #[test]
    fn the_clone_reproduces_the_reference_pitch() {
        let backend = AcousticCloneBackend::new();
        let profile = backend
            .create_profile(&reference(120.0, 16_000), None)
            .expect("create profile");

        let request = TtsRequest {
            text: "aaa".into(),
            voice_id: profile.id.clone(),
            speed: 1.0,
        };
        let cloned = backend.synthesize_with(&profile, &request).expect("clone");

        let neutral = backend
            .synthesize(&TtsRequest {
                text: "aaa".into(),
                voice_id: MODEL_ID.into(),
                speed: 1.0,
            })
            .expect("neutral");

        // The stored voiceprint must carry the reference's pitch.
        let print = VoicePrint::from_embedding(profile.cached_embedding.as_ref().unwrap());
        assert!(
            (print.f0_hz - 120.0).abs() < 8.0,
            "voiceprint pitch should be ~120 Hz, got {:.2}",
            print.f0_hz
        );

        // And the rendered audio must actually carry that pitch: re-analyze
        // the synthesized waveform with the same tracker used on references.
        let clone_pitch = analyze_voiceprint(&cloned);
        assert!(
            clone_pitch.frames > 4,
            "the clone must contain voiced frames, got {}",
            clone_pitch.frames
        );
        assert!(
            (clone_pitch.f0_hz - 120.0).abs() < 12.0,
            "the clone should sound near the reference's 120 Hz, got {:.2}",
            clone_pitch.f0_hz
        );

        // The neutral voice uses the default 160 Hz pitch, so the clone must
        // sit measurably below it.
        let neutral_pitch = analyze_voiceprint(&neutral);
        assert!(
            clone_pitch.f0_hz < neutral_pitch.f0_hz,
            "the clone ({:.1} Hz) must be lower than the neutral voice ({:.1} Hz)",
            clone_pitch.f0_hz,
            neutral_pitch.f0_hz
        );
    }

    #[test]
    fn synthesis_produces_audible_audio() {
        let backend = AcousticCloneBackend::new();
        let pcm = backend
            .synthesize(&TtsRequest {
                text: "Hello from Audio Labs Pro".into(),
                voice_id: MODEL_ID.into(),
                speed: 1.0,
            })
            .expect("synthesis");
        assert_eq!(pcm.sample_rate(), SAMPLE_RATE);
        assert_eq!(pcm.channels(), 1);
        assert!(pcm.frames() > 10_000);
        assert!(
            pcm.peak() > 0.05,
            "expected audible output, got {}",
            pcm.peak()
        );
    }

    #[test]
    fn the_voiceprint_of_the_clone_matches_its_source() {
        let backend = AcousticCloneBackend::new();
        let profile = backend
            .create_profile(&reference(180.0, 16_000), None)
            .expect("create profile");
        let embedding = profile.cached_embedding.as_ref().expect("embedding");
        let restored = VoicePrint::from_embedding(embedding);
        let direct = analyze_voiceprint(&reference(180.0, 16_000));
        assert!((restored.f0_hz - direct.f0_hz).abs() < 1.0);
    }
}
