//! MoFA Studio - Main entry point
//!
//! Parses command-line arguments and starts the application.
//!
//! # Usage
//!
//! ```bash
//! mofa-studio --help          # Show help
//! mofa-studio --dark-mode     # Start in dark mode
//! mofa-studio --log-level debug  # Enable debug logging
//! ```

mod app;
mod cli;

pub use cli::Args;

use clap::Parser;

fn main() {
    // Parse command-line arguments
    let args = Args::parse();

    // Configure logging based on CLI args
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(args.log_filter()))
        .init();

    // Install panic hook to capture panic message before extern "C" abort
    std::panic::set_hook(Box::new(|info| {
        eprintln!("=== PANIC CAPTURED ===");
        eprintln!("{}", info);
        if let Some(loc) = info.location() {
            eprintln!("Location: {}:{}:{}", loc.file(), loc.line(), loc.column());
        }
        eprintln!("Backtrace:\n{}", std::backtrace::Backtrace::force_capture());
        eprintln!("=== END PANIC ===");
    }));

    log::info!("Starting MoFA Studio");
    log::debug!("CLI args: {:?}", args);

    if args.dark_mode {
        log::info!("Dark mode enabled via CLI");
    }

    if let Some(ref dataflow) = args.dataflow {
        log::info!("Using dataflow: {}", dataflow);
    }

    // Store args for app to access
    app::set_cli_args(args);

    // Start the application
    app::app_main();
}
