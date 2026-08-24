use std::{net::SocketAddr, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "eni", version, about = "ElectronNotIncluded runtime tools")]
pub struct Cli {
    #[arg(long, global = true, default_value = "assets/data")]
    pub data_dir: PathBuf,
    #[arg(long, global = true)]
    pub headless: bool,
    #[arg(
        long,
        global = true,
        help = "run the REST server alongside the runtime"
    )]
    pub serve: bool,
    #[arg(long, global = true, default_value = "127.0.0.1:3000")]
    pub bind: SocketAddr,
    #[arg(long, global = true, value_enum, default_value_t = StartState::Menu)]
    pub start_state: StartState,
    #[arg(long, global = true, default_value_t = false)]
    pub debug: bool,
    #[arg(long, global = true, default_value_t = 0.0)]
    pub seconds: f64,
    #[arg(long, global = true, help = "override world generation seed")]
    pub seed: Option<u32>,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Verify,
    Headless {
        #[arg(long, default_value_t = 0.0)]
        seconds: f64,
    },
    Render {
        #[arg(long, default_value = "target/preview/world_runtime.png")]
        output: PathBuf,
    },
    Operate {
        #[arg(value_enum)]
        operation: Operation,
        #[arg(long, default_value_t = 0.0)]
        seconds: f64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Operation {
    AdvanceTime,
    Pause,
    Resume,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum StartState {
    #[default]
    Menu,
    Playing,
}

#[cfg(test)]
mod tests {
    use super::{Cli, Command, Operation, StartState};
    use clap::Parser;

    #[test]
    fn parses_verify_command() {
        let cli = Cli::try_parse_from(["eni", "verify"]).expect("verify should parse");
        assert!(matches!(cli.command, Some(Command::Verify)));
    }

    #[test]
    fn parses_operate_command() {
        let cli = Cli::try_parse_from(["eni", "operate", "advance-time", "--seconds", "12.5"])
            .expect("operate should parse");
        assert!(matches!(
            cli.command,
            Some(Command::Operate {
                operation: Operation::AdvanceTime,
                seconds
            }) if (seconds - 12.5).abs() < f64::EPSILON
        ));
    }

    #[test]
    fn parses_serve_as_a_global_runtime_option() {
        let cli = Cli::try_parse_from(["eni", "--headless", "--serve", "--bind", "127.0.0.1:3317"])
            .expect("serve options should parse without a serve command");
        assert!(cli.headless);
        assert!(cli.serve);
        assert_eq!(cli.bind.to_string(), "127.0.0.1:3317");
        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_headless_start_state() {
        let cli = Cli::try_parse_from(["eni", "--headless", "--start-state", "playing"])
            .expect("headless start state should parse");
        assert_eq!(cli.start_state, StartState::Playing);
    }
}
