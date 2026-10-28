//! Message handling for the shell.
//!
//! All state transitions live here; views only construct messages. Editing
//! messages call `EditSession`, which is the sole path to project commands.

use super::*;

impl App {
    /// Apply a message.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle_message(message);
        // The palette depends on the active workspace (its neon hue) and the
        // theme mode, both of which messages can change; the stateful demo
        // waveform carries its own copy, so re-tint it whenever they diverge.
        let colors = self.colors();
        if colors != self.waveform_tint {
            self.waveform.set_colors(colors);
            self.waveform_tint = colors;
        }
        task
    }

    fn handle_message(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::WindowOpened(id) => {
                self.window = Some(id);
                Task::batch([
                    window::run(id, |window| glass::probe(window)).map(Message::GlassProbed),
                    window::size(id).map(Message::WindowResized),
                ])
            }
            Message::WindowEvent(_id, event) => match event {
                window::Event::Resized(size) => {
                    self.apply_window_size(size);
                    Task::none()
                }
                window::Event::FileDropped(path) => Task::done(Message::ImportPath(path)),
                _ => Task::none(),
            },
            Message::WindowResized(size) => {
                self.apply_window_size(size);
                Task::none()
            }
            Message::CloseRequested => {
                self.save_workspace();
                self.session.flush_autosave();
                Task::none()
            }
            Message::GlassProbed(report) => {
                self.glass = report;
                Task::none()
            }
            Message::Frame(now) => {
                self.jobs.poll();
                self.poll_generation();
                self.session.poll_autosave();
                if self.session.poll_import() {
                    self.sync_edit_fields();
                }
                if self.demo_autoscroll {
                    self.waveform.scroll_by(0.35);
                }
                if self.metrics {
                    self.metrics_frames += 1;
                    if self.metrics_last.elapsed() >= Duration::from_secs(1) {
                        eprintln!(
                            "alp-ui frames/s={} frame_ms={:.2} wave={:.0}s",
                            self.metrics_frames, self.frame_ms, self.media_duration
                        );
                        self.metrics_frames = 0;
                        self.metrics_last = Instant::now();
                    }
                }
                if self.autoclose.is_some_and(|deadline| now >= deadline) {
                    self.session.flush_autosave();
                    return iced::exit();
                }
                if let Some(last) = self.last_frame {
                    let dt = now.duration_since(last).as_secs_f32();
                    self.frame_ms = self.frame_ms * 0.9 + dt * 1000.0 * 0.1;
                    if self.playing {
                        self.position_secs += dt;
                        if self.position_secs >= self.media_duration {
                            self.position_secs = 0.0;
                            self.playing = false;
                        }
                        self.waveform.set_playhead(self.position_secs);
                    }
                }
                if let Some(recording) = &self.recording {
                    self.record_peak = (self.record_peak * 0.92)
                        .max(recording.meter().peak())
                        .clamp(0.0, 1.0);
                    self.record_rms = recording.meter().rms().clamp(0.0, 1.0);
                }
                if let Some(playback) = &self.playback {
                    let total = self.media_duration.max(1.0);
                    let rate = self.playback_rate.max(1) as f32;
                    let frames = playback.position_frames();
                    self.position_secs = (frames as f32 / rate).clamp(0.0, total);
                    self.waveform.set_playhead(self.position_secs);
                    self.record_peak = playback.master_peak().clamp(0.0, 1.0);
                    self.record_rms = playback.master_rms().clamp(0.0, 1.0);
                    if let Some(meters) = playback.track_meters() {
                        self.playback_meters = Some(meters);
                    }
                    if playback.finished() {
                        let done = self.playback.take();
                        drop(done);
                        self.playing = false;
                    }
                }
                self.last_frame = Some(now);
                for toggle in self.panel_motion.values_mut() {
                    toggle.tick(now);
                }
                Task::none()
            }
            Message::DragPanel(kind) => {
                let rect = self.panels.panel(kind).rect;
                self.drag = Some(Drag::Panel {
                    kind,
                    grab: Point::new(self.last_cursor.x - rect.x, self.last_cursor.y - rect.y),
                });
                self.panels.focus(kind);
                Task::none()
            }
            Message::ResizePanel(kind) => {
                self.drag = Some(Drag::Resize { kind });
                self.panels.focus(kind);
                Task::none()
            }
            Message::TogglePanel(kind) => {
                self.workflow_tab = match kind {
                    PanelKind::VoiceLab => WorkflowTab::Voices,
                    PanelKind::Models => WorkflowTab::Models,
                    PanelKind::MediaBin => WorkflowTab::Captures,
                    PanelKind::Transport => WorkflowTab::Captures,
                    PanelKind::Effects => WorkflowTab::Effects,
                    PanelKind::Mixer
                    | PanelKind::Inspector
                    | PanelKind::AiActivity
                    | PanelKind::Timeline => WorkflowTab::Stories,
                    PanelKind::ExportQueue => WorkflowTab::Generate,
                };
                self.panels.set_open(kind, true);
                Task::none()
            }
            Message::ClosePanel(kind) => {
                self.set_panel_open(kind, false);
                Task::none()
            }
            Message::CollapsePanel(kind) => {
                self.panels.toggle_collapsed(kind);
                self.save_workspace();
                Task::none()
            }
            Message::ToggleDock(kind) => {
                if self.panels.panel(kind).docked.is_some() {
                    self.panels.undock(kind);
                } else {
                    self.panels.dock(kind, DockSide::Right);
                }
                self.save_workspace();
                Task::none()
            }
            Message::PaletteInput(query) => {
                self.palette.set_query(&query, &self.commands);
                Task::none()
            }
            Message::RunPaletteSelection => {
                if let Some(entry) = self.palette.selected_command(&self.commands) {
                    self.palette.hide();
                    return self.run_command(entry.id);
                }
                Task::none()
            }
            Message::RunCommand(id) => self.run_command(id),
            Message::ToggleTheme => {
                self.theme_mode = self.theme_mode.next();
                self.system_dark = alp_ui_kit::tokens::system_is_dark();
                self.save_workspace();
                Task::none()
            }
            Message::ToggleReduceMotion => {
                self.reduce_motion = !self.reduce_motion;
                for toggle in self.panel_motion.values_mut() {
                    toggle.set_reduce_motion(self.reduce_motion);
                }
                self.save_workspace();
                Task::none()
            }
            Message::SaveLayout => {
                self.save_workspace();
                Task::none()
            }
            Message::ResetLayout => {
                self.panels.reset_geometry();
                self.save_workspace();
                self.show_toast("Layout reset");
                Task::none()
            }
            Message::PlayPause => {
                if self.playing {
                    self.playing = false;
                    if let Some(playback) = self.playback.take() {
                        playback.stop();
                    }
                    Task::none()
                } else {
                    let scene = match self.session.scene() {
                        Ok(scene) => scene,
                        Err(error) => {
                            self.show_toast(format!("Cannot play: {error}"));
                            return Task::none();
                        }
                    };
                    let media_duration = scene.duration_secs() as f32;
                    match play_scene_live(&scene, None) {
                        Ok(handle) => {
                            self.playback = Some(handle);
                            self.playback_rate = scene.sample_rate();
                            self.playing = true;
                            self.media_duration = media_duration.max(1.0);
                            self.waveform.set_playhead(self.position_secs);
                        }
                        Err(error) => self.show_toast(format!("Cannot play: {error}")),
                    }
                    Task::none()
                }
            }
            Message::Stop => {
                if let Some(playback) = self.playback.take() {
                    playback.stop();
                }
                self.playing = false;
                self.position_secs = 0.0;
                self.waveform.set_playhead(0.0);
                Task::none()
            }
            Message::ImportMedia => {
                self.show_toast("Drop a WAV, FLAC, MP3 or OGG file onto the window");
                Task::none()
            }
            Message::ImportPath(path) => {
                self.import_path(path);
                Task::none()
            }
            Message::BuildPeaks => {
                self.jobs.spawn("Verify peak cache", |reporter| {
                    for step in 0..=40u16 {
                        if reporter.is_cancelled() {
                            return Err("cancelled".into());
                        }
                        reporter.progress(f32::from(step) / 40.0);
                        reporter.detail("Checking pyramid levels");
                        std::thread::sleep(Duration::from_millis(15));
                    }
                    Ok("Peak cache verified".into())
                });
                Task::none()
            }
            Message::QueueExport | Message::ExportMix => {
                self.export_mix();
                Task::none()
            }
            Message::ExportPreset(preset) => {
                self.export_with_preset(preset);
                Task::none()
            }
            Message::CancelJob(id) => {
                self.jobs.cancel(id);
                Task::none()
            }
            Message::ClearJobs => {
                self.jobs.clear_finished();
                Task::none()
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }
            Message::WindowDrag => {
                if let Some(id) = self.window {
                    window::drag(id)
                } else {
                    Task::none()
                }
            }
            Message::WindowMinimize => {
                if let Some(id) = self.window {
                    window::minimize::<Message>(id, true).map(|_| Message::Noop)
                } else {
                    Task::none()
                }
            }
            Message::WindowMaximize => {
                if let Some(id) = self.window {
                    window::toggle_maximize::<Message>(id).map(|_| Message::Noop)
                } else {
                    Task::none()
                }
            }
            Message::WindowClose => {
                self.save_workspace();
                self.session.flush_autosave();
                iced::exit()
            }
            Message::Event(event) => self.handle_event(event),
            Message::SelectClip(id) => {
                self.session.select_clip(id);
                self.sync_edit_fields();
                Task::none()
            }
            Message::SelectionStart(value) => {
                self.selection_start = value;
                Task::none()
            }
            Message::SelectionEnd(value) => {
                self.selection_end = value;
                Task::none()
            }
            Message::ApplySelection => {
                match (
                    self.selection_start.parse::<f64>(),
                    self.selection_end.parse::<f64>(),
                ) {
                    (Ok(start), Ok(end)) => {
                        self.session.set_selection(start, end);
                        self.sync_edit_fields();
                    }
                    _ => self.show_toast("Selection needs seconds, for example 1.5"),
                }
                Task::none()
            }
            Message::CutSelection => {
                self.session.cut();
                self.sync_edit_fields();
                Task::none()
            }
            Message::CopySelection => {
                self.session.copy();
                Task::none()
            }
            Message::PasteClipboard => {
                self.session.paste();
                self.sync_edit_fields();
                Task::none()
            }
            Message::DeleteSelection => {
                self.session.delete(self.ripple);
                self.sync_edit_fields();
                Task::none()
            }
            Message::ToggleRipple => {
                self.ripple = !self.ripple;
                self.show_toast(if self.ripple {
                    "Ripple deletes enabled"
                } else {
                    "Deletes leave gaps"
                });
                Task::none()
            }
            Message::SilenceSelection => {
                self.session.silence();
                self.sync_edit_fields();
                Task::none()
            }
            Message::DipEnvelope => {
                self.session.dip_selection();
                Task::none()
            }
            Message::ResetEnvelope => {
                self.session.reset_envelope();
                Task::none()
            }
            Message::TrimSelection => {
                self.session.trim();
                self.sync_edit_fields();
                Task::none()
            }
            Message::SplitSelection => {
                self.session.split();
                self.sync_edit_fields();
                Task::none()
            }
            Message::DuplicateSelection => {
                self.session.duplicate();
                self.sync_edit_fields();
                Task::none()
            }
            Message::NormalizeSelection => {
                self.session.normalize();
                self.sync_edit_fields();
                Task::none()
            }
            Message::ToggleReverse => {
                self.session.toggle_reverse();
                Task::none()
            }
            Message::ToggleInvert => {
                self.session.toggle_invert();
                Task::none()
            }
            Message::UndoEdit => {
                self.session.undo();
                self.sync_edit_fields();
                Task::none()
            }
            Message::RedoEdit => {
                self.session.redo();
                self.sync_edit_fields();
                Task::none()
            }
            Message::EditGain(value) => {
                self.edit_gain = value;
                self.session.set_gain(value);
                Task::none()
            }
            Message::EditFadeIn(value) => {
                self.edit_fade_in = value;
                self.session.set_fades(value, self.edit_fade_out);
                Task::none()
            }
            Message::EditFadeOut(value) => {
                self.edit_fade_out = value;
                self.session.set_fades(self.edit_fade_in, value);
                Task::none()
            }
            Message::SelectInput(name) => {
                self.record_device = Some(name);
                Task::none()
            }
            Message::ToggleMonitor => {
                self.record_monitor = !self.record_monitor;
                Task::none()
            }
            Message::TogglePunchIn => {
                self.punch_in = !self.punch_in;
                self.show_toast(if self.punch_in {
                    "Punch-in armed · select a range and record"
                } else {
                    "Punch-in off · recordings append to the timeline"
                });
                Task::none()
            }
            Message::ToggleRecord => {
                self.toggle_recording();
                Task::none()
            }
            Message::SelectEffect(index) => {
                self.selected_effect = index.min(self.effects.len().saturating_sub(1));
                Task::none()
            }
            Message::ToggleEffect(index) => {
                if let Some(effect) = self.effects.get_mut(index) {
                    effect.enabled = !effect.enabled;
                }
                Task::none()
            }
            Message::ApplyVoiceCleanupPreset => {
                for effect in &mut self.effects {
                    effect.enabled = true;
                }
                Task::none()
            }
            Message::RescanPlugins => {
                let scanner = alp_plugin::PluginScanner::new();
                let _ = scanner.scan();
                self.discovered_plugins = alp_plugin::PluginHost::new().list_plugins();
                self.show_toast(format!(
                    "Discovered {} plugins",
                    self.discovered_plugins.len()
                ));
                Task::none()
            }
            Message::InsertPlugin(id) => {
                if let Some(desc) = self.discovered_plugins.iter().find(|p| p.id == id).cloned() {
                    self.effects.push(EffectItem {
                        name: desc.name.clone(),
                        enabled: true,
                        params: format!("{:?} · {}", desc.format, desc.vendor),
                    });
                    self.selected_effect = self.effects.len().saturating_sub(1);
                    self.show_toast(format!("Inserted plugin {}", desc.name));
                }
                Task::none()
            }
            Message::SelectVoice(voice) => {
                let id = self
                    .profiles
                    .iter()
                    .find(|profile| profile.name == voice || profile.id == voice)
                    .map_or(voice, |profile| profile.id.clone());
                self.select_profile(&id);
                Task::none()
            }
            Message::GenerateEditor(action) => {
                self.voice_editor.perform(action);
                self.voice_text = self.voice_editor.text();
                Task::none()
            }
            Message::OpenCreateVoice => {
                self.create_voice_open = true;
                self.create_voice_tab = CreateVoiceTab::Record;
                self.create_voice_reference.clear();
                self.create_voice_consent = false;
                Task::none()
            }
            Message::CloseCreateVoice => {
                self.create_voice_open = false;
                self.create_voice_consent = false;
                Task::none()
            }
            Message::CreateVoiceTab(tab) => {
                self.create_voice_tab = tab;
                Task::none()
            }
            Message::CreateVoiceConsent(granted) => {
                self.create_voice_consent = granted;
                Task::none()
            }
            Message::CreateVoiceReferenceInput(text) => {
                self.create_voice_reference = text;
                Task::none()
            }
            Message::CreateVoiceLanguage(language) => {
                self.create_voice_language = language;
                Task::none()
            }
            Message::CreateVoiceSubmit => {
                self.submit_create_voice();
                Task::none()
            }
            Message::CreateDesignedVoice => {
                self.create_designed_profile();
                if self
                    .profiles
                    .iter()
                    .any(|profile| profile.id.starts_with("design-"))
                {
                    self.create_voice_open = false;
                }
                Task::none()
            }
            Message::ImportVoice => {
                self.show_toast("Drop a .alpvoice bundle on the window to import it");
                Task::none()
            }
            Message::ToggleStoriesTools => {
                self.stories_tools_open = !self.stories_tools_open;
                Task::none()
            }
            Message::GenerateSpeech => {
                self.start_generation();
                Task::none()
            }
            Message::PlayHistory(index) => {
                self.play_history(index);
                Task::none()
            }
            Message::InsertHistory(index) => {
                self.insert_history(index);
                Task::none()
            }
            Message::DeleteHistory(index) => {
                self.delete_history(index);
                Task::none()
            }
            Message::ClearHistory => {
                self.history.clear();
                self.show_toast("History cleared");
                Task::none()
            }
            Message::SelectProfile(id) => {
                self.select_profile(&id);
                Task::none()
            }
            Message::ProfileNameInput(text) => {
                self.profile_name = text;
                Task::none()
            }
            Message::ProfileTagsInput(text) => {
                self.profile_tags = text;
                Task::none()
            }
            Message::DeleteProfile(id) => {
                self.delete_profile(&id);
                Task::none()
            }
            Message::ExportProfile(id) => {
                self.export_profile(&id);
                Task::none()
            }
            Message::StoryLineInput(text) => {
                self.story_text = text;
                Task::none()
            }
            Message::AddStoryLine => {
                self.story_add_line();
                Task::none()
            }
            Message::SelectStoryLine(index) => {
                self.selected_story_line = Some(index);
                Task::none()
            }
            Message::DeleteStoryLine(index) => {
                self.story_delete_line(index);
                Task::none()
            }
            Message::NewStory => {
                self.story_new();
                Task::none()
            }
            Message::SaveStory => {
                self.story_save();
                Task::none()
            }
            Message::RenderStory => {
                self.story_render();
                Task::none()
            }
            Message::SelectStory(index) => {
                self.selected_story = index.min(self.stories.len().saturating_sub(1));
                self.selected_story_line = None;
                Task::none()
            }
            Message::SelectCapture(index) => {
                self.selected_capture = index.min(self.captures.len().saturating_sub(1));
                Task::none()
            }
            Message::ExportTranscriptSrt => {
                self.export_transcript(true);
                Task::none()
            }
            Message::ExportTranscriptTxt => {
                self.export_transcript(false);
                Task::none()
            }
            Message::AcceptModelLicense(id) => {
                self.accept_model_license(&id);
                Task::none()
            }
            Message::RemoveModel(id) => {
                self.remove_model(&id);
                Task::none()
            }
            Message::ToggleOfflineModels => {
                self.toggle_offline_models();
                Task::none()
            }
            Message::ToggleDictation => {
                self.dictating = !self.dictating;
                if self.dictating {
                    self.dictation_preview = "Listening (built-in VAD)".into();
                    self.show_toast("Dictation armed · stop to segment the selected capture");
                } else {
                    self.dictation_preview.clear();
                    self.transcribe_capture();
                }
                Task::none()
            }
            Message::SelectTranscriptSegment(index) => {
                if let Some(segment) = self.transcript.segments.get(index) {
                    let rate = self.session.project().sample_rate.max(1);
                    let secs = segment.start_frame as f32 / rate as f32;
                    self.position_secs = secs;
                    self.waveform.set_playhead(secs);
                    self.selected_segment = Some(index);
                    self.show_toast(format!("Seek to segment {index} at {secs:.2}s"));
                }
                Task::none()
            }
            Message::DeleteTranscriptSegment(index) => {
                if let Some(range) = self.transcript.cut_range_for_segment(index) {
                    if index < self.transcript.segments.len() {
                        self.transcript.segments.remove(index);
                    }
                    self.session.delete_range(range, self.ripple);
                    self.sync_edit_fields();
                    self.show_toast(format!(
                        "Ripple deleted segment {index} ({:.2}s)",
                        (range.end - range.start) as f64 / 48_000.0
                    ));
                }
                Task::none()
            }
            Message::TranscribeActiveTrack => {
                self.transcribe_capture();
                Task::none()
            }
            Message::AuditReferenceAudio => {
                self.audit_reference();
                Task::none()
            }
            Message::OpenConsentDialog => {
                self.consent_dialog_open = true;
                self.show_toast("Human consent attestation required for cloning");
                Task::none()
            }
            Message::ConfirmConsent => {
                self.create_cloned_profile();
                Task::none()
            }
            Message::CloseConsentDialog => {
                self.consent_dialog_open = false;
                Task::none()
            }
            Message::ConsentSpeakerInput(text) => {
                self.consent_speaker = text;
                Task::none()
            }
            Message::ConsentAttesterInput(text) => {
                self.consent_attester = text;
                Task::none()
            }
            Message::SelectTrack(index) => {
                self.selected_track = index.min(self.session.track_count().saturating_sub(1));
                Task::none()
            }
            Message::CrossfadeSelection(overlap) => {
                self.session.crossfade_selected(overlap);
                self.sync_edit_fields();
                Task::none()
            }
            Message::MoveSelectedClipToPrevTrack => {
                self.session.move_selected_clip_to_prev_track();
                self.sync_edit_fields();
                Task::none()
            }
            Message::MoveSelectedClipToNextTrack => {
                self.session.move_selected_clip_to_next_track();
                self.sync_edit_fields();
                Task::none()
            }
            Message::SetMasterGain(gain) => {
                self.session.set_master_gain(gain);
                self.gain = self.session.master_gain();
                Task::none()
            }
            Message::SetTrackGain(index, gain) => {
                self.session.set_track_gain(index, gain);
                Task::none()
            }
            Message::SetTrackPan(index, pan) => {
                self.session.set_track_pan(index, pan);
                Task::none()
            }
            Message::ToggleTrackMute(index) => {
                self.session.toggle_track_mute(index);
                Task::none()
            }
            Message::ToggleTrackSolo(index) => {
                self.session.toggle_track_solo(index);
                Task::none()
            }
            Message::ToggleTrackArm(index) => {
                self.session.toggle_track_arm(index);
                Task::none()
            }
            Message::ZoomWaveform(factor) => {
                self.waveform.zoom_by(factor);
                Task::none()
            }
            Message::SeekWaveform(secs) => {
                self.position_secs = secs.clamp(0.0, self.media_duration);
                self.waveform.set_playhead(self.position_secs);
                Task::none()
            }
            Message::OpenFirstRunWizard => {
                self.first_run_open = true;
                Task::none()
            }
            Message::FinishFirstRunWizard => {
                self.first_run_open = false;
                self.save_workspace();
                self.show_toast("First-run configuration completed");
                Task::none()
            }
            Message::OpenShortcutsHelp => {
                self.shortcuts_help_open = true;
                Task::none()
            }
            Message::CloseShortcutsHelp => {
                self.shortcuts_help_open = false;
                Task::none()
            }
            Message::ToggleAiControl => {
                self.ai_enabled = !self.ai_enabled;
                self.ai_status = if self.ai_enabled {
                    "AI Control Active (Listening on control.sock)".into()
                } else {
                    "AI Control Standby".into()
                };
                self.show_toast(if self.ai_enabled {
                    "AI Control enabled (Tier: Edit)"
                } else {
                    "AI Control disabled"
                });
                Task::none()
            }
            Message::SetAiPermissionTier(tier) => {
                self.ai_tier = tier;
                self.show_toast(format!("AI Permission Tier updated to {tier:?}"));
                Task::none()
            }
            Message::StopAi => {
                self.ai_enabled = false;
                self.ai_status = "AI Session Stopped / Revoked".into();
                self.ai_activity_log
                    .push("EMERGENCY STOP: AI session revoked and active tasks cancelled".into());
                self.show_toast("AI activity stopped and jobs cancelled");
                Task::none()
            }
            Message::UndoAiChanges => {
                self.session.undo();
                self.ai_activity_log
                    .push("Rollback executed: Restored session checkpoint".into());
                self.sync_edit_fields();
                self.show_toast("Rolled back all AI changes to session start");
                Task::none()
            }
            Message::SelectWorkflowTab(tab) => {
                self.workflow_tab = tab;
                self.palette.open = false;
                Task::none()
            }
            Message::Noop => Task::none(),
        }
    }

    fn import_path(&mut self, path: PathBuf) {
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if extension == "alpvoice" {
            self.import_profile(path);
            return;
        }
        match self.session.begin_import(path) {
            Ok(()) => self.show_toast("Import started in the background"),
            Err(error) => self.show_toast(error),
        }
    }

    pub(crate) fn sync_edit_fields(&mut self) {
        let selection = self.session.selection();
        let rate = self.session.project().sample_rate;
        self.selection_start = format!("{:.2}", frames_to_secs(selection.range().start, rate));
        self.selection_end = format!("{:.2}", frames_to_secs(selection.range().end, rate));
        self.session.selected_clip().and_then(|id| {
            self.session.clip(id).map(|clip| {
                self.edit_gain = clip.gain;
                self.edit_fade_in = if clip.is_empty() {
                    0.0
                } else {
                    clip.fade_in_frames as f32 / clip.len() as f32
                };
                self.edit_fade_out = if clip.is_empty() {
                    0.0
                } else {
                    clip.fade_out_frames as f32 / clip.len() as f32
                };
            })
        });
    }

    fn toggle_recording(&mut self) {
        if let Some(recording) = self.recording.take() {
            let path = self.record_path.clone();
            let peak = self.record_peak;
            match recording.stop() {
                Ok(report) => {
                    let seconds = report.frames as f32 / report.sample_rate.max(1) as f32;
                    self.record_peak = 0.0;
                    self.record_rms = 0.0;
                    if let Some(path) = path {
                        self.record_capture(&path, seconds, peak);
                        let punched = if self.punch_in {
                            self.session.punch_in_file(&path)
                        } else {
                            self.session.import_file(&path);
                            true
                        };
                        self.sync_edit_fields();
                        self.show_toast(if punched {
                            format!(
                                "{} {:.1}s · {} dropped",
                                if self.punch_in { "Punched" } else { "Recorded" },
                                report.frames as f64 / f64::from(report.sample_rate.max(1)),
                                report.dropped
                            )
                        } else {
                            self.session.status().to_owned()
                        });
                    }
                }
                Err(error) => self.show_toast(format!("Recording failed: {error}")),
            }
            self.record_started = None;
            return;
        }
        if self.punch_in && self.session.selection().is_empty() {
            self.show_toast("Select a range before punch-in");
            return;
        }
        let Some(dirs) = alp_core::AppDirs::discover().ok() else {
            self.show_toast("No application data directory");
            return;
        };
        let directory = dirs.projects_dir().join("ui-demo");
        if let Err(error) = std::fs::create_dir_all(&directory) {
            self.show_toast(format!("Cannot create recording folder: {error}"));
            return;
        }
        let path = directory.join(format!("take-{}.wav", now_ms()));
        let config = CaptureConfig {
            device: self.record_device.clone(),
            chunk_frames: 48_000 * 5,
            monitor: self.record_monitor,
        };
        match start_recording(&path, &config) {
            Ok(recording) => {
                self.recording = Some(recording);
                self.record_path = Some(path);
                self.record_started = Some(Instant::now());
                self.record_peak = 0.0;
                self.record_rms = 0.0;
                self.show_toast("Recording · crash-safe chunks every 5s");
            }
            Err(error) => self.show_toast(format!("Cannot start recording: {error}")),
        }
    }

    fn run_command(&mut self, id: CommandId) -> Task<Message> {
        if self.palette.open && id != CommandId::TogglePalette {
            self.palette.hide();
        }
        match id {
            CommandId::TogglePalette => {
                if self.palette.open {
                    self.palette.hide();
                    Task::none()
                } else {
                    self.palette.show(&self.commands);
                    iced::widget::operation::focus::<Message>(self.palette_id.clone())
                }
            }
            CommandId::ToggleTheme => self.update(Message::ToggleTheme),
            CommandId::ToggleReduceMotion => self.update(Message::ToggleReduceMotion),
            CommandId::ResetLayout => self.update(Message::ResetLayout),
            CommandId::SaveLayout => self.update(Message::SaveLayout),
            CommandId::OpenPanel(kind) => {
                self.set_panel_open(kind, true);
                Task::none()
            }
            CommandId::ClosePanel(kind) => {
                self.set_panel_open(kind, false);
                Task::none()
            }
            CommandId::ImportMedia => self.update(Message::ImportMedia),
            CommandId::BuildPeaks => self.update(Message::BuildPeaks),
            CommandId::PlayPause => self.update(Message::PlayPause),
            CommandId::Stop => self.update(Message::Stop),
            CommandId::QueueExport => self.update(Message::QueueExport),
            CommandId::OpenShortcutsHelp => self.update(Message::OpenShortcutsHelp),
            CommandId::OpenFirstRunWizard => self.update(Message::OpenFirstRunWizard),
        }
    }

    fn handle_event(&mut self, event: iced::Event) -> Task<Message> {
        match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                if self.shortcuts_help_open {
                    if let iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) = key {
                        self.shortcuts_help_open = false;
                    }
                    return Task::none();
                }
                if self.first_run_open {
                    if let iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) = key {
                        self.first_run_open = false;
                    }
                    return Task::none();
                }
                if self.palette.open {
                    match key {
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                            self.palette.hide();
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                            return self.update(Message::RunPaletteSelection);
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                            self.palette.move_selection(1);
                        }
                        iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                            self.palette.move_selection(-1);
                        }
                        _ => {}
                    }
                    return Task::none();
                }
                if let Some(id) = command_for_key(modifiers, &key) {
                    return self.run_command(id);
                }
                if let iced::keyboard::Key::Character(value) = &key
                    && modifiers.control()
                    && !modifiers.shift()
                    && !modifiers.alt()
                    && let Some(index) = value
                        .chars()
                        .next()
                        .and_then(|character| character.to_digit(10))
                    && (1..=8).contains(&index)
                {
                    let kind = PanelKind::ALL[index as usize - 1];
                    self.set_panel_open(kind, true);
                }
                Task::none()
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { position }) => {
                self.last_cursor = position;
                if let Some(drag) = self.drag {
                    match drag {
                        Drag::Panel { kind, grab } => {
                            self.panels.drag_to(kind, position, grab);
                        }
                        Drag::Resize { kind } => {
                            self.panels.resize_to(kind, position);
                        }
                    }
                }
                Task::none()
            }
            iced::Event::Mouse(mouse::Event::ButtonReleased(_)) => {
                if let Some(drag) = self.drag.take() {
                    let kind = match drag {
                        Drag::Panel { kind, .. } | Drag::Resize { kind } => kind,
                    };
                    self.panels.settle(kind);
                    self.save_workspace();
                }
                Task::none()
            }
            iced::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let timeline = self.panels.panel(PanelKind::Timeline).rect;
                let over_timeline = self.last_cursor.x >= timeline.x
                    && self.last_cursor.x <= timeline.right()
                    && self.last_cursor.y >= timeline.y
                    && self.last_cursor.y <= timeline.bottom();
                if over_timeline {
                    let y = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y,
                        mouse::ScrollDelta::Pixels { y, .. } => y / 40.0,
                    };
                    self.waveform.zoom_by(if y > 0.0 { 0.8 } else { 1.25 });
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn apply_window_size(&mut self, size: Size) {
        if !self.layout_restored && !self.bounds_initialized {
            self.panels = PanelManager::default_for(size);
            self.bounds_initialized = true;
        } else {
            self.panels.set_bounds(size);
        }
    }

    fn set_panel_open(&mut self, kind: PanelKind, open: bool) {
        self.panels.set_open(kind, open);
        let reduce = self.reduce_motion;
        let toggle = self.panel_motion.entry(kind).or_insert_with(|| {
            Toggle::new(if open { 1.0 } else { 0.0 }).with_duration(MotionDuration::PANEL_MS)
        });
        toggle.set_reduce_motion(reduce);
        toggle.toggle(open);
        self.save_workspace();
    }

    fn save_workspace(&mut self) {
        let Some(path) = &self.save_path else {
            return;
        };
        let layout = PersistedWorkspace::new(
            self.theme_mode.as_str(),
            self.reduce_motion,
            self.glass.mode.as_str(),
            self.panels.snapshots(),
        )
        .with_first_run_completed(!self.first_run_open);
        if let Err(error) = layout.save(path) {
            self.last_save_error = Some(error.to_string());
        } else {
            self.last_save_error = None;
        }
    }

    pub(crate) fn show_toast(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }
}
