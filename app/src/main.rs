//! awesome-superb-soundboard — host companion app.
//!
//! Stub for now. The accompanying desktop app talks to the macropad over USB
//! (HID/serial) to push sample banks, change key mappings, and read encoder
//! state. This skeleton gives us a compilable binary to build on.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "soundboard-app",
    about = "Host companion app for the a.s.s. RP2040 macropad"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List macropads currently attached over USB.
    List,
    /// Print firmware/board status.
    Status,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::List) => {
            println!("No macropads found (stub).");
        }
        Some(Commands::Status) => {
            println!("soundboard-app 0.1.0 — not yet connected to firmware.");
        }
        None => {
            println!("soundboard-app ready. Try `soundboard-app list` or `status`.");
        }
    }
}
