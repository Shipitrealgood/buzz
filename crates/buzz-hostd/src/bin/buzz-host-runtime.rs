//! Fixed installed same-unit runtime; never accepts caller paths or commands.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    buzz_hostd::runtime_context::run()
        .await
        .map_err(|_| anyhow::anyhow!("managed runtime unavailable"))
}
