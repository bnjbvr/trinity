use anyhow;
use trinity;

async fn real_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    tracing::debug!("parsing config...");
    let config = trinity::BotConfig::from_env()?;

    tracing::debug!("creating client...");
    trinity::run(config).await
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // just one trick to get rust-analyzer working in main :-)
    real_main().await
}
