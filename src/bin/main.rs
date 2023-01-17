use anyhow;
use trinity::BotConfig;

async fn real_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let config_path = std::env::args().nth(1);

    tracing::debug!("parsing config...");
    // First check for a config file, then fallback to env if none found.
    let config = BotConfig::from_config(config_path).or_else(|_| BotConfig::from_env())?;

    tracing::debug!("creating client...");
    trinity::run(config).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // just one trick to get rust-analyzer working in main :-)
    real_main().await
}
