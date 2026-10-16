#[tokio::main]
async fn main() -> anyhow::Result<()> {
    aio_plugin_dictionary_server::hosting::run().await
}
