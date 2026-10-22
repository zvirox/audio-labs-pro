//! Command dispatch for `alp-cli`.

pub mod device;
pub mod media;
pub mod project;

use anyhow::Result;

use crate::cli::Command;

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
            bit_depth,
        } => project::render(&project, &output, &bit_depth),
        Command::Play { project, seconds } => project::play(&project, seconds),
        Command::Devices { json } => device::devices(json),
        Command::Record { output, seconds } => device::record(&output, seconds),
    }
}
