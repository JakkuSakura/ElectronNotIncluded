//! ElectronNotIncluded desktop launcher.

use bevy::app::ScheduleRunnerPlugin;
use bevy::prelude::*;
use bevy::window::ExitCondition;
use bevy::winit::WinitPlugin;
use clap::Parser;
use eni_client::{
    ClientControlQueue, ClientPlugin, DebugMode, FramebufferCaptureQueue, HeadlessMode,
    HeadlessStartState,
};
use eni_domain::{ChunkCoord, DataError, DomainPlugin, GameData, generate_chunk};
use eni_server::ServerPlugin;
use serde::Serialize;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

mod cli;
mod runtime;

use cli::{Cli, Command, StartState};
use runtime::RuntimeError;

#[derive(Debug, Error)]
enum StartupError {
    #[error("failed to load game data: {0}")]
    Data(#[from] DataError),
    #[error("runtime error: {0}")]
    Runtime(#[from] RuntimeError),
}

fn main() -> Result<(), StartupError> {
    let cli = Cli::parse();
    if cli.headless && !cli.serve {
        let snapshot = runtime::run_headless(&cli.data_dir, cli.seed, cli.seconds)?;
        print_json(&snapshot)?;
        return Ok(());
    }
    match cli.command {
        Some(Command::Verify) => {
            let summary = runtime::verify(&cli.data_dir, cli.seed)?;
            print_json(&summary)?;
            return Ok(());
        }
        Some(Command::Headless { seconds }) => {
            let snapshot = runtime::run_headless(&cli.data_dir, cli.seed, seconds)?;
            print_json(&snapshot)?;
            return Ok(());
        }
        Some(Command::Render { output }) => {
            let game_data = GameData::load(&cli.data_dir)?;
            let seed = cli.seed.unwrap_or(runtime::DEFAULT_SEED);
            let chunk = generate_chunk(seed, ChunkCoord { x: 0, y: 0 }, &game_data.substances);
            runtime::save_world_png(&chunk, &output)?;
            println!("rendered world image to {}", output.display());
            return Ok(());
        }
        Some(Command::Operate { operation, seconds }) => {
            let snapshot = runtime::operate(&cli.data_dir, cli.seed, operation, seconds)?;
            print_json(&snapshot)?;
            return Ok(());
        }
        None => {}
    }
    let game_data = GameData::load(&cli.data_dir)?;
    let seed = cli.seed.unwrap_or_else(|| {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        now.as_secs() as u32
    });
    tracing::info!(seed, "world generation seed");
    let primary_window = if cli.headless {
        None
    } else {
        Some(Window {
            title: "ElectronNotIncluded".into(),
            resolution: (960, 640).into(),
            visible: true,
            ..default()
        })
    };
    let mut app = App::new();
    if cli.serve {
        let (screenshot_sender, screenshot_receiver) = mpsc::channel();
        let (control_sender, control_receiver) = mpsc::channel();
        runtime::spawn_server(
            cli.data_dir.clone(),
            Some(seed),
            cli.bind,
            Some(screenshot_sender),
            Some(control_sender),
        )
        .map_err(StartupError::Runtime)?;
        app.insert_resource(FramebufferCaptureQueue(Arc::new(Mutex::new(
            screenshot_receiver,
        ))));
        app.insert_resource(ClientControlQueue(Arc::new(Mutex::new(control_receiver))));
    }
    if cli.headless {
        let start_state = match cli.start_state {
            StartState::Menu => HeadlessStartState::Menu,
            StartState::Playing => HeadlessStartState::Playing,
        };
        app.insert_resource(HeadlessMode { start_state });
        app.add_plugins(ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
    }
    let mut default_plugins = DefaultPlugins
        .set(AssetPlugin {
            file_path: "../../assets".into(),
            ..default()
        })
        .set(WindowPlugin {
            primary_window,
            exit_condition: if cli.headless {
                ExitCondition::DontExit
            } else {
                ExitCondition::OnAllClosed
            },
            ..default()
        });
    if cli.headless {
        default_plugins = default_plugins.disable::<WinitPlugin>();
    }
    app.insert_resource(game_data);
    if cli.debug {
        app.insert_resource(DebugMode);
    }
    app.add_plugins(default_plugins)
        .add_plugins(DomainPlugin)
        .add_plugins(ServerPlugin { seed })
        .add_plugins(ClientPlugin)
        .run();
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<(), StartupError> {
    let output = serde_json::to_string_pretty(value).map_err(RuntimeError::Json)?;
    println!("{output}");
    Ok(())
}
