//! CLI entry point for blog-rs.
//!
//! Provides two subcommands:
//! - `build` — generate the static site into the output directory
//! - `serve` — build and serve locally via `tiny_http`

mod config;
mod post;
mod utils;
mod render;
mod serve;
mod site;
mod template;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "blog-rs", about = "A Rust-powered static blog engine")]
enum Cli {
    /// Build the static site
    Build {
        /// Output directory
        #[arg(short, long, default_value = "public")]
        output: Option<PathBuf>,
    },
    /// Build and serve the site locally
    Serve {
        /// Port to serve on
        #[arg(short, long, default_value_t = 3000)]
        port: u16,
    },
}

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let config = config::SiteConfig::load(PathBuf::from("config.toml").as_path())?;

    match cli {
        Cli::Build { .. } => {
            site::build(&config)?;
            println!("Site built successfully!");
        }
        Cli::Serve { port } => {
            site::build(&config)?;
            let output_dir = config.output_dir.clone();
            println!("Site built. Starting server...");
            serve::serve(
                std::path::Path::new(&output_dir),
                port,
                move || site::build(&config),
            )?;
        }
    }

    Ok(())
}
