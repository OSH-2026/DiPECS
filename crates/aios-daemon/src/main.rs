//! dipecsd — 入口点。实际逻辑在 lib.rs。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    aios_daemon::run().await
}
