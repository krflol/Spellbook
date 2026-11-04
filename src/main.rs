#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use std::path::PathBuf;

/// Spellbook — an offline World of Warcraft macro IDE.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Directory for the local macro library and settings.
    #[arg(long)]
    data_dir: Option<PathBuf>,
    /// Store data beside the executable in a data folder.
    #[arg(long, conflicts_with = "data_dir")]
    portable: bool,
    /// Import a macro text file or JSON library on launch.
    #[arg(value_name = "FILE")]
    files: Vec<PathBuf>,
    /// Render native demonstration screenshots using isolated data, then exit.
    #[arg(long, hide = true)]
    capture_dir: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let result = (|| -> anyhow::Result<()> {
        let data_dir = if let Some(capture) = &args.capture_dir {
            capture.join(".capture-data")
        } else {
            spellbook::session::data_dir(args.data_dir, args.portable)?
        };
        spellbook::desktop::run(data_dir, args.files, args.capture_dir)
    })();
    if let Err(error) = result {
        eprintln!("Spellbook: {error:#}");
        rfd::MessageDialog::new()
            .set_title("Spellbook could not start")
            .set_description(format!("{error:#}"))
            .set_level(rfd::MessageLevel::Error)
            .show();
        std::process::exit(1);
    }
}
