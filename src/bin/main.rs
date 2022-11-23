use trinity;
use anyhow;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // just one trick to get rust-analyzer working in main :-)
    trinity::real_main().await
}
