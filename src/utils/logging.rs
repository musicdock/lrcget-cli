use anyhow::Result;
use tracing::Level;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init_logging(verbose: bool, suppress_logs: bool) -> Result<()> {
    // If logs are suppressed (Terminal UI mode), don't initialize tracing subscriber
    if suppress_logs {
        return Ok(());
    }

    let level = if verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy()
        // Filter out noisy dependencies (these parse strings are static and known-valid)
        .add_directive("reqwest=warn".parse().expect("valid directive for reqwest"))
        .add_directive("rusqlite=warn".parse().expect("valid directive for rusqlite"))
        .add_directive("lofty=warn".parse().expect("valid directive for lofty"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(false)
        .with_line_number(false)
        .compact()
        .init();

    Ok(())
}