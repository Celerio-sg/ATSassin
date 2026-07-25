use anyhow::Result;
use atsassin::cli::Cli;
use atsassin::engine::hardware::HardwareProfile;
use clap::Parser;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Parse CLI first — --help/--version exit here without triggering
    // the expensive hardware probe (~400ms PowerShell CIM on Windows).
    let cli = Cli::parse();

    // Hardware detection only runs when actually executing a command.
    let profile = HardwareProfile::global();
    info!(
        "Hardware detected: {}GB RAM, {} cores, GPU: {}",
        profile.total_ram_gb,
        profile.cpu_cores,
        if profile.has_gpu {
            format!("yes ({}GB VRAM)", profile.gpu_vram_gb.unwrap_or(0))
        } else {
            "no".to_string()
        }
    );
    for hint in profile.cpu_optimization_hints() {
        info!("HW: {}", hint);
    }

    cli.run().await
}
