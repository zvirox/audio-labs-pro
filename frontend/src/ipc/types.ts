/**
 * Audio Labs Pro - Tauri IPC Type Contract
 *
 * Generated and synchronized with alp_session and alp_api models.
 * Used for typed communication between React frontend and alp-desktop.
 */

export interface ProjectSummary {
  id: string;
  name: string;
  revision: number;
  sample_rate: number;
  duration_secs: number;
  track_count: number;
  clip_count: number;
  undo_steps: number;
  redo_steps: number;
  is_dirty: boolean;
}

export interface TrackSummary {
  id: string;
  name: string;
  index: number;
  gain: number;
  pan: number;
  muted: boolean;
  solo: boolean;
  armed: boolean;
  clip_count: number;
}

export interface PanelRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type DockSide = 'left' | 'right';

export type PanelKind =
  | 'mixer'
  | 'transport'
  | 'timeline'
  | 'inspector'
  | 'media_bin'
  | 'voice_lab'
  | 'effects'
  | 'models'
  | 'export_queue'
  | 'ai_activity';

export interface PanelState {
  kind: PanelKind;
  rect: PanelRect;
  z: number;
  collapsed: boolean;
  docked: DockSide | null;
  open: boolean;
}

export interface WorkspaceState {
  version: number;
  theme: string;
  reduce_motion: boolean;
  glass: string;
  panels: PanelState[];
  approved_ai_clients?: string[];
  first_run_completed: boolean;
}

export interface GlassReport {
  session: string;
  desktop: string;
  handle: string;
  mode: 'compositor_blur' | 'tint_fallback';
  blur_available: boolean;
  detail: string;
}

export interface SessionRevisionPayload {
  revision: number;
  is_dirty: boolean;
  reason: string;
}

export interface IpcError {
  code: string;
  message: string;
}

export interface MeterFrame {
  peak_l: number;
  peak_r: number;
  rms_l: number;
  rms_r: number;
  position_secs: number;
  anchor_timestamp_ms: number;
  playing: boolean;
  duration_secs: number;
}

export interface TransportState {
  playing: boolean;
  position_secs: number;
  position_frames: number;
  duration_secs: number;
  sample_rate: number;
  loop_enabled: boolean;
  loop_start_secs: number;
  loop_end_secs: number;
  volume: number;
  master_peak_l: number;
  master_peak_r: number;
  master_rms_l: number;
  master_rms_r: number;
  anchor_timestamp_ms: number;
}

export interface PeakLevelDto {
  bucket_samples: number;
  pairs: [number, number][];
}

export interface WaveformPeaksDto {
  sample_rate: number;
  frames: number;
  duration_secs: number;
  levels: PeakLevelDto[];
}

export interface AudioDevice {
  name: string;
  is_default: boolean;
  direction: 'input' | 'output';
  channels: number;
  sample_rate: number;
  sample_format: string;
}

export interface ModelItem {
  id: string;
  name: string;
  kind: 'tts_light' | 'tts_medium' | 'stt' | 'cloning';
  size_mb: number;
  status: 'installed' | 'verified' | 'downloading' | 'not_installed' | 'corrupt';
  progress?: number;
  license: string;
  spdx: string;
  description: string;
  is_builtin: boolean;
}

export interface ConsentRecord {
  speaker_name: string;
  attested_by: string;
  timestamp: number;
  statement: string;
}

export interface VoiceProfile {
  id: string;
  name: string;
  language: string;
  tags: string[];
  kind: 'acoustic' | 'prebuilt' | 'cloned';
  speed: number;
  pitch: number;
  energy: number;
  emotion: string;
  consent?: ConsentRecord;
}

export interface EffectStage {
  id: string;
  name: string;
  category: string;
  enabled: boolean;
  params: Record<string, number>;
}

export interface BiquadBandDto {
  filter_type: string;
  frequency_hz: number;
  q: number;
  gain_db: number;
  enabled: boolean;
}

export interface BiquadResponseDto {
  frequencies_hz: number[];
  magnitude_db: number[];
  phase_deg: number[];
}

export interface DiscoveredPlugin {
  id: string;
  name: string;
  vendor: string;
  format: 'CLAP' | 'LV2' | 'VST3';
  category: string;
}

export interface CaptureTake {
  id: string;
  name: string;
  duration_secs: number;
  timestamp: string;
  peak_db: number;
  rms_db: number;
  clipped: boolean;
}

export interface TranscriptWord {
  word: string;
  start_secs: number;
  end_secs: number;
  confidence: number;
  speaker: string;
}

export interface StoryLine {
  id: string;
  speaker: string;
  text: string;
  status: 'pending' | 'rendering' | 'ready' | 'failed';
  take_duration?: number;
}

export interface StoryScript {
  id: string;
  title: string;
  chapter: number;
  lines: StoryLine[];
}

export interface AcxChapterFileDto {
  title: string;
  audio_path: string;
}

export interface AcxChapterQa {
  title: string;
  rms_dbfs: number;
  true_peak_dbfs: number;
  noise_floor_dbfs: number;
  compliant: boolean;
  failures: string[];
}

export interface AcxQaReport {
  chapters: AcxChapterQa[];
  compliant_count: number;
  total_count: number;
  all_compliant: boolean;
}

export interface M4bChapterDto {
  title: string;
  start_secs: number;
  duration_secs: number;
}

export interface M4bMetadataDto {
  title: string;
  narrator: string;
  series: string;
  series_number: number | null;
  comment: string;
}

export interface M4bExportRequestDto {
  output_path: string;
  master_audio_path: string;
  chapters: M4bChapterDto[];
  cover_path: string | null;
  metadata: M4bMetadataDto;
}

export interface M4bExportReport {
  path: string;
  frames: number;
  duration_secs: number;
  bitrate: number;
  chapter_count: number;
  has_cover: boolean;
}

export interface StemSeparationRequestDto {
  audio_path: string;
  output_dir: string;
}

export interface StemFileDto {
  name: string;
  path: string;
}

export interface StemSeparationReport {
  stems: StemFileDto[];
  sample_rate: number;
}

export interface RateFitRequestDto {
  audio_path: string;
  target_duration_secs: number;
  output_path: string;
}

export interface FitPlanDto {
  current_duration_secs: number;
  target_duration_secs: number;
  recommended_speed: number;
  stretch_ratio: number;
  uses_wsola: boolean;
  within_tts_range: boolean;
}

export interface RateFitReport {
  path: string;
  duration_secs: number;
  plan: FitPlanDto;
}

export interface VideoRemuxRequestDto {
  video_path: string;
  audio_path: string | null;
  output_path: string;
  subtitle_path: string | null;
}

export interface RemuxReport {
  output_path: string;
  duration_secs: number | null;
  video_stream_copied: boolean;
  audio_codec: string;
}

export interface MacroStepDto {
  effect_id: string;
  params: Record<string, number>;
}

export interface MacroRequestDto {
  input_dir: string;
  output_dir: string;
  pattern?: string | null;
  overwrite?: boolean;
  steps: MacroStepDto[];
}

export interface MacroFileResultDto {
  input: string;
  output: string;
  ok: boolean;
  error: string | null;
}

export interface MacroReportDto {
  processed: number;
  succeeded: number;
  failed: number;
  files: MacroFileResultDto[];
  cancelled: boolean;
}

export interface MacroJobReportDto {
  job_id: string;
  report: MacroReportDto;
}

export type PermissionTier = 'observe' | 'edit' | 'export';

export interface AiControlStatus {
  enabled: boolean;
  socket_path: string | null;
  token: string | null;
  tier: PermissionTier;
  trusted_clients: string[];
  pending_clients: string[];
  commits: number;
  last_commit_ms: number | null;
  max_commit_ms: number;
  last_status: string;
}

export interface AiActivityEntry {
  timestamp: string;
  client_id: string;
  operation: string;
  outcome: string;
  queue_to_commit_ms: number | null;
  committed: boolean;
}

export interface AiApprovalRequest {
  clientId: string;
}

export interface SpectrogramData {
  sample_rate: number;
  duration_sec: number;
  num_frames: number;
  num_bins: number;
  freq_bins_hz: number[];
  time_stamps_sec: number[];
  matrix: number[];
  min_db: number;
  max_db: number;
}

export * from '../timeline/types';
