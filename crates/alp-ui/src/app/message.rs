//! Top-level shell messages and modal sub-tabs.

use std::path::PathBuf;
use std::time::Instant;

use alp_core::ClipId;
use iced::widget::text_editor;
use iced::{Size, window};

use super::workspace_layout::WorkflowTab;
use crate::commands::CommandId;
use crate::glass::GlassReport;
use crate::panels::PanelKind;

/// Source tab in the Create Voice dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CreateVoiceTab {
    /// Import a sample from disk.
    Upload,
    /// Record a sample with the selected input.
    #[default]
    Record,
    /// Capture system audio.
    SystemAudio,
}

impl CreateVoiceTab {
    /// Index used by the segmented control.
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Upload => 0,
            Self::Record => 1,
            Self::SystemAudio => 2,
        }
    }
}

/// Top-level shell messages.
#[derive(Debug, Clone)]
pub enum Message {
    /// iced opened the main window.
    WindowOpened(window::Id),
    /// A window event (resize, focus, close).
    WindowEvent(window::Id, window::Event),
    /// The window's current inner size.
    WindowResized(Size),
    /// Primary monitor size, used to fit the initial window.
    MonitorSized(Option<Size>),
    /// Close request from the compositor.
    CloseRequested,
    /// Glass probe completed.
    GlassProbed(GlassReport),
    /// Raw input event.
    Event(iced::Event),
    /// Animation frame.
    Frame(Instant),
    /// Start dragging a panel.
    #[allow(dead_code)]
    DragPanel(PanelKind),
    /// Start resizing a panel.
    #[allow(dead_code)]
    ResizePanel(PanelKind),
    /// Toggle a panel.
    #[allow(dead_code)]
    TogglePanel(PanelKind),
    /// Close a panel.
    #[allow(dead_code)]
    ClosePanel(PanelKind),
    /// Toggle collapse.
    #[allow(dead_code)]
    CollapsePanel(PanelKind),
    /// Dock or undock a panel.
    #[allow(dead_code)]
    ToggleDock(PanelKind),
    /// Palette text changed.
    PaletteInput(String),
    /// Run the highlighted palette command.
    RunPaletteSelection,
    /// Run a command by id.
    RunCommand(CommandId),
    /// Theme cycle.
    ToggleTheme,
    /// Reduce motion.
    ToggleReduceMotion,
    /// Save layout.
    SaveLayout,
    /// Reset layout.
    ResetLayout,
    /// Play/pause.
    PlayPause,
    /// Stop and rewind.
    Stop,
    /// Import demo media.
    ImportMedia,
    /// Import a file dropped onto the window.
    ImportPath(PathBuf),
    /// Rebuild peaks (worker stub).
    BuildPeaks,
    /// Queue a demo export.
    QueueExport,
    /// Cancel a job.
    CancelJob(u64),
    /// Remove finished jobs.
    ClearJobs,
    /// Dismiss the toast.
    DismissToast,
    /// Window title drag.
    WindowDrag,
    /// Minimize.
    WindowMinimize,
    /// Toggle maximize.
    WindowMaximize,
    /// Close button.
    WindowClose,
    /// Select a clip in the editing track.
    SelectClip(ClipId),
    /// Selection start text.
    SelectionStart(String),
    /// Selection end text.
    SelectionEnd(String),
    /// Apply the typed selection.
    ApplySelection,
    /// Cut the selection.
    CutSelection,
    /// Copy the selection.
    CopySelection,
    /// Paste the clipboard.
    PasteClipboard,
    /// Delete the selection or selected clip.
    DeleteSelection,
    /// Toggle ripple deletes.
    ToggleRipple,
    /// Silence the selection.
    SilenceSelection,
    /// Lower the clip envelope over the selection.
    DipEnvelope,
    /// Clear the selected clip envelope.
    ResetEnvelope,
    /// Trim the selected clip to the selection.
    TrimSelection,
    /// Split the selected clip at the cursor.
    SplitSelection,
    /// Duplicate the selected clip.
    DuplicateSelection,
    /// Normalize the selected clip.
    NormalizeSelection,
    /// Reverse the selected clip.
    ToggleReverse,
    /// Invert the selected clip.
    ToggleInvert,
    /// Undo the last edit.
    UndoEdit,
    /// Redo the next edit.
    RedoEdit,
    /// Clip gain for the selected clip.
    EditGain(f32),
    /// Clip fade-in fraction.
    EditFadeIn(f32),
    /// Clip fade-out fraction.
    EditFadeOut(f32),
    /// Select an input device.
    SelectInput(String),
    /// Toggle input monitoring.
    ToggleMonitor,
    /// Toggle recording into the selected range instead of appending.
    TogglePunchIn,
    /// Start or stop a recording.
    ToggleRecord,
    /// Export the current mix.
    ExportMix,
    /// Export using an industry preset.
    ExportPreset(alp_media::ExportPreset),
    /// Select an effect row.
    SelectEffect(usize),
    /// Toggle an effect's enabled state.
    ToggleEffect(usize),
    /// Apply voice cleanup preset across effects chain.
    ApplyVoiceCleanupPreset,
    /// Select a voice row.
    SelectVoice(String),
    /// Select a track.
    SelectTrack(usize),
    /// Crossfade the selected clip with the following clip.
    CrossfadeSelection(f32),
    /// Move the selected clip to the previous track.
    MoveSelectedClipToPrevTrack,
    /// Move the selected clip to the next track.
    MoveSelectedClipToNextTrack,
    /// Set the master bus gain.
    SetMasterGain(f32),
    /// Set a track's linear gain.
    SetTrackGain(usize, f32),
    /// Set a track's pan position.
    SetTrackPan(usize, f32),
    /// Toggle a track's mute state.
    ToggleTrackMute(usize),
    /// Toggle a track's solo state.
    ToggleTrackSolo(usize),
    /// Toggle a track's record-arm state.
    ToggleTrackArm(usize),
    /// Zoom waveform in/out.
    ZoomWaveform(f32),
    /// Seek waveform.
    SeekWaveform(f32),
    /// Toggle live dictation.
    ToggleDictation,
    /// Select and seek to a transcript segment.
    SelectTranscriptSegment(usize),
    /// Delete a transcript segment and ripple cut from timeline.
    DeleteTranscriptSegment(usize),
    /// Transcribe speech audio in the active track.
    TranscribeActiveTrack,
    /// Audit selected reference audio for cloning quality.
    AuditReferenceAudio,
    /// Open the human consent attestation dialog.
    OpenConsentDialog,
    /// Confirm and store human consent attestation.
    ConfirmConsent,
    /// Close the human consent dialog.
    CloseConsentDialog,
    /// Consent speaker-name field.
    ConsentSpeakerInput(String),
    /// Consent attester-name field.
    ConsentAttesterInput(String),
    /// Open first-run configuration wizard.
    OpenFirstRunWizard,
    /// Finish and dismiss first-run configuration wizard.
    FinishFirstRunWizard,
    /// Open keyboard shortcuts help modal.
    OpenShortcutsHelp,
    /// Close keyboard shortcuts help modal.
    CloseShortcutsHelp,
    /// Toggle AI automation master switch.
    ToggleAiControl,
    /// A control-host event waiting for the shared UI command path.
    AiControl(alp_api::control::ControlEvent),
    /// Decide whether a newly attached client is trusted.
    ResolveAiApproval(alp_api::control::ApprovalDecision),
    /// Copy the enabled host's one-session authentication token.
    CopyAiToken,
    /// Set the permission tier for AI clients.
    SetAiPermissionTier(alp_api::PermissionTier),
    /// Immediately stop all AI automation and cancel running jobs.
    StopAi,
    /// Roll back all changes made by AI automation in the current session.
    UndoAiChanges,
    /// Switch active workflow workspace tab.
    SelectWorkflowTab(WorkflowTab),
    /// Switch active horizontal subtab within the current workspace.
    SelectSubtab(usize),
    /// Toggle the right-hand collapsible inspector panel.
    ToggleInspector,
    /// Edit the Generate tab prompt in the multi-line editor.
    GenerateEditor(text_editor::Action),
    /// Open the Create Voice dialog.
    OpenCreateVoice,
    /// Close the Create Voice dialog.
    CloseCreateVoice,
    /// Switch the Create Voice sample source.
    CreateVoiceTab(CreateVoiceTab),
    /// Reference transcript typed into the Create Voice dialog.
    CreateVoiceReferenceInput(String),
    /// Speaker-permission declaration toggled in the Create Voice dialog.
    CreateVoiceConsent(bool),
    /// Target language chosen in the Create Voice dialog.
    CreateVoiceLanguage(String),
    /// Create the profile described by the dialog.
    CreateVoiceSubmit,
    /// Create a designed profile from the dialog description.
    CreateDesignedVoice,
    /// Open the import hint for voice bundles.
    ImportVoice,
    /// Show or hide the mixer/inspector tools over the Stories editor.
    ToggleStoriesTools,
    /// Queue speech generation for the selected profile.
    GenerateSpeech,
    /// Play a generated take from history.
    PlayHistory(usize),
    /// Insert a generated take into the timeline.
    InsertHistory(usize),
    /// Remove a generated take from history.
    DeleteHistory(usize),
    /// Clear all generated takes.
    ClearHistory,
    /// Select a voice profile by id.
    SelectProfile(String),
    /// Voice profile display-name field.
    ProfileNameInput(String),
    /// Voice profile description/tags field.
    ProfileTagsInput(String),
    /// Delete a voice profile from the library.
    DeleteProfile(String),
    /// Export a profile as a `.alpvoice` bundle.
    ExportProfile(String),
    /// Story line text field.
    StoryLineInput(String),
    /// Append the typed line to the current story.
    AddStoryLine,
    /// Select a story line for editing.
    SelectStoryLine(usize),
    /// Delete a story line.
    DeleteStoryLine(usize),
    /// Create a new empty story.
    NewStory,
    /// Save the current story as JSON.
    SaveStory,
    /// Render every pending story line into one take.
    RenderStory,
    /// Select a story by index.
    SelectStory(usize),
    /// Select a capture row.
    SelectCapture(usize),
    /// Export the transcript as SRT.
    ExportTranscriptSrt,
    /// Export the transcript as plain text.
    ExportTranscriptTxt,
    /// Accept a model license.
    AcceptModelLicense(String),
    /// Remove an installed model from disk.
    RemoveModel(String),
    /// Start (or resume) a background download for a model.
    DownloadModel(String),
    /// Ask the alp-infer daemon to load a model as its active engine.
    LoadNeuralModel(String),
    /// Probe the alp-infer daemon for its current engine state.
    CheckEngine,
    /// Toggle offline-only model mode.
    ToggleOfflineModels,
    /// Rescan available CLAP and LV2 audio plugins.
    RescanPlugins,
    /// Insert a discovered plugin onto the active track's effect chain.
    InsertPlugin(String),
    /// No-op task result.
    Noop,
}
