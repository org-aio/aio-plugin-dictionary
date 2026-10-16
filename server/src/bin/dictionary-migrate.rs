use anyhow::{Context, Result, ensure};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    ensure!(
        args.len() == 1,
        "用法: dictionary-migrate <tenant-id>；先停止旧宿主并备份，设置 AIO_LEGACY_DATABASE_URL 和 AIO_COMPONENT_DATABASE_URL"
    );
    let legacy = std::env::var("AIO_LEGACY_DATABASE_URL").context("缺少旧宿主数据库")?;
    let storage = std::env::var("AIO_COMPONENT_DATABASE_URL").context("缺少组件数据库")?;
    let host = PgPoolOptions::new()
        .max_connections(2)
        .connect(&legacy)
        .await?;
    let target = PgPoolOptions::new()
        .max_connections(2)
        .connect(&storage)
        .await?;
    let result =
        aio_plugin_dictionary_server::migration::copy_legacy(&host, &target, &args[0]).await;
    host.close().await;
    target.close().await;
    let report = result?;
    println!(
        "已核对并复制 {} 个类型、{} 个字典项；旧表保留。",
        report.types, report.items
    );
    Ok(())
}
