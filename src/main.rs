use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

mod parser;
mod token;
mod char_ext;
mod tape;
mod etc;
mod slice_ext;
mod r#macro;

#[derive(Parser, Debug)]
#[command(name = "mgc")]
#[command(version = "0.1.0")]
#[command(about = "Malgam (mgc): High-performance programmable markup compiler.", long_about = None)]
struct Args {
    /// The input file or directory to compile.
    /// Defaults to the current directory if not provided.
    #[arg(value_name = "PATH", default_value = ".")]
    input: PathBuf,

    /// Explicitly set the output directory.
    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    /// Change the working directory before running.
    #[arg(short = 'C', long, value_name = "PATH")]
    workdir: Option<PathBuf>,

    /// Override a configuration setting (e.g., -D finance-mode=true).
    /// Can be used multiple times.
    #[arg(short = 'D', value_name = "KEY=VALUE")]
    config_override: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // 1. Handle Workdir Override (like `git -C`)
    if let Some(ref new_dir) = args.workdir {
        std::env::set_current_dir(new_dir)
            .with_context(|| format!("Failed to change directory to {:?}", new_dir))?;
    }

    // 2. Logic to determine if input is File or Dir
    if args.input.is_dir() {
        println!("Processing all files in directory: {:?}", args.input);
    } else {
        println!("Processing single file: {:?}", args.input);
    }

    // 3. Process Config Overrides
    for entry in args.config_override {
        if let Some((key, value)) = entry.split_once('=') {
            println!("Overriding config: {} => {}", key, value);
        }
    }

    Ok(())
}
