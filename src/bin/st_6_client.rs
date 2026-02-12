use core::time;

use mini_runtime::{
    config::{self, CRLF},
    result::Result,
    sleep,
    web::client::ClientBuilder,
};

#[rt_entry::main]
async fn main() -> Result<()> {
    a().await
}

async fn a() -> Result<()> {
    log::info!("client start");
    let client = ClientBuilder::new(config::ECHO_SERVER_IP, config::ECHO_SERVER_PORT)
        .update_timeout(|timeout| {
            timeout
                .update_timeout(time::Duration::from_secs(10))
                .set_read_timeout(time::Duration::from_secs(5));
        })
        .connect()?;
    sleep(time::Duration::from_millis(3000)).await;

    client
        .writer()
        .lock()
        .await
        .send(format!("Civilization VI!{}", CRLF).as_bytes())
        .await?;

    let body = client.reader().read_until_exclusive(CRLF).await?;
    log::info!("resp: {:?}", String::from_utf8_lossy(&body));
    Ok(())
}
