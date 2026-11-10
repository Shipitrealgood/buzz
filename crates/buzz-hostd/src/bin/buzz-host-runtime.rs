//! Fixed installed same-unit runtime; never accepts caller paths or commands.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let result = if args.is_empty() {
        buzz_hostd::runtime_context::run().await
    } else if args.len() == 3 && args[0] == "--user-service" {
        buzz_hostd::runtime_context::run_user(
            std::path::Path::new(&args[1]),
            args[2]
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("invalid unit ID"))?,
        )
        .await
    } else {
        Err(anyhow::anyhow!("invalid runtime invocation"))
    };
    result.map_err(|_| anyhow::anyhow!("managed runtime unavailable"))
}
