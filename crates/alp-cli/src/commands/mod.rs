//! Command dispatch for `alp-cli`.

pub mod device;
pub mod dsp;
pub mod media;
pub mod plugin;
pub mod profile;
pub mod project;
pub mod stt;
pub mod voice;

use anyhow::Result;

use crate::cli::{Command, ModelCommand, PluginCommand, ProfileCommand, ScriptCommand};

/// Run one parsed command.
pub fn dispatch(command: Command) -> Result<()> {
    match command {
        Command::Gen {
            output,
            seconds,
            sample_rate,
            channels,
            frequency,
            amplitude,
        } => media::generate(
            &output,
            seconds,
            sample_rate,
            channels,
            frequency,
            amplitude,
        ),
        Command::Decode { input, json } => media::decode_file(&input, json),
        Command::Peaks {
            input,
            cache_dir,
            json,
        } => media::peaks(&input, cache_dir, json),
        Command::New {
            directory,
            name,
            sample_rate,
        } => project::create(&directory, name, sample_rate),
        Command::Info { project, json } => project::info(&project, json),
        Command::Import {
            project,
            input,
            track,
            at,
            name,
        } => project::import(&project, &input, track, at, name),
        Command::Render {
            project,
            output,
            preset,
            format,
            bit_depth,
            sample_rate,
            dither,
            title,
            artist,
            album,
            comment,
        } => project::render(
            &project,
            &output,
            project::RenderOptions {
                preset: preset.as_deref(),
                format: &format,
                bit_depth: &bit_depth,
                sample_rate,
                dither: &dither,
                metadata: alp_media::AudioMetadata {
                    title,
                    artist,
                    album,
                    comment,
                },
            },
        ),
        Command::RenderStems {
            project,
            output_dir,
            preset,
            format,
            bit_depth,
            dither,
            title,
            artist,
            album,
            comment,
        } => project::render_stems(
            &project,
            output_dir.as_deref(),
            project::RenderOptions {
                preset: preset.as_deref(),
                format: &format,
                bit_depth: &bit_depth,
                sample_rate: None,
                dither: &dither,
                metadata: alp_media::AudioMetadata {
                    title,
                    artist,
                    album,
                    comment,
                },
            },
        ),
        Command::Play { project, seconds } => project::play(&project, seconds),
        Command::Devices { json } => device::devices(json),
        Command::Record {
            output,
            seconds,
            device,
            monitor,
            chunk_seconds,
        } => device::record(&output, seconds, device.as_deref(), monitor, chunk_seconds),
        Command::Recover { output, json } => device::recover(&output, json),
        Command::DeleteRange {
            project,
            start,
            end,
            no_ripple,
        } => project::delete_range(&project, start, end, !no_ripple),
        Command::Autosave { project } => project::autosave(&project),
        Command::RecoverProject { project, json } => project::recover_project(&project, json),
        Command::NullTest {
            reference,
            rendered,
            tolerance,
            json,
        } => media::null_test(&reference, &rendered, tolerance, json),
        Command::Lufs { input, json } => dsp::measure(&input, json),
        Command::Cleanup {
            input,
            output,
            mains_freq,
        } => dsp::cleanup(&input, &output, mains_freq),
        Command::Tts {
            text,
            output,
            voice,
            speed,
        } => voice::tts(&text, &output, &voice, speed),
        Command::Model(model_cmd) => match model_cmd {
            ModelCommand::List { json } => voice::model_list(json),
            ModelCommand::Verify { model } => voice::model_verify(&model),
            ModelCommand::AcceptLicense { model } => voice::model_accept_license(&model),
        },
        Command::Script(script_cmd) => match script_cmd {
            ScriptCommand::Render {
                script,
                output,
                stitch,
            } => voice::script_render(&script, &output, stitch),
        },
        Command::Transcribe {
            input,
            out,
            format,
            json,
        } => stt::transcribe(&input, out.as_deref(), &format, json),
        Command::Vad {
            input,
            threshold_db,
            hangover_ms,
            min_speech_ms,
            json,
        } => stt::vad(&input, threshold_db, hangover_ms, min_speech_ms, json),
        Command::Profile(prof_cmd) => match prof_cmd {
            ProfileCommand::List { json } => profile::list(json),
            ProfileCommand::Info { id, json } => profile::info(&id, json),
            ProfileCommand::CheckReference { input, json } => {
                profile::check_reference(&input, json)
            }
            ProfileCommand::Design {
                name,
                prompt,
                tags,
                json,
            } => profile::design(&name, &prompt, tags, json),
            ProfileCommand::Export { id, output } => profile::export_profile(&id, &output),
            ProfileCommand::Import { input, json } => profile::import_profile(&input, json),
            ProfileCommand::Compare {
                reference,
                generated,
                json,
            } => profile::compare(&reference, &generated, json),
            ProfileCommand::Delete { id } => profile::delete(&id),
        },
        Command::Plugin(plugin_cmd) => match plugin_cmd {
            PluginCommand::Scan { path, json } => plugin::scan(path.as_deref(), json),
            PluginCommand::List { json } => plugin::list(json),
            PluginCommand::Inspect { id, json } => plugin::inspect(&id, json),
        },
        Command::Hardware { json } => profile::hardware_probe(json),
    }
}
