#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    qm_example_server::start().await?;
    Ok(())
}
