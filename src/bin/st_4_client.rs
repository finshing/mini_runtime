use std::time;

use mini_runtime::{
    config,
    result::Result,
    sleep,
    sync::wait_group::{WaitGroup, WaitGroupGuard},
    web::client::ClientBuilder,
};

#[rt_entry::main]
async fn main() -> Result<()> {
    let start_at = time::Instant::now();
    let wg = WaitGroup::new();
    for i in 0..100i64 {
        // spawn!(call((i - 5).pow(2) * 10, wg.add()));
        spawn!(call2(i as usize, wg.add()));
    }

    wg.wait().await;
    log::info!("total cost {}ms", start_at.elapsed().as_millis());
    Ok(())
}

#[allow(unused)]
async fn call(dur: i64, _guard: WaitGroupGuard<'_>) -> Result<()> {
    let start_at = time::Instant::now();
    let client = ClientBuilder::new(config::ECHO_SERVER_IP, config::ECHO_SERVER_PORT).connect()?;
    client
        .writer()
        .lock()
        .await
        .send("hello, echo server".as_bytes())
        .await?;
    log::info!("send body");

    sleep(time::Duration::from_millis(dur as u64)).await;
    let resp = client.reader().readall().await?;
    let resp_str = String::from_utf8(resp);
    log::info!(
        "get response: {:?}, const {}ms",
        resp_str,
        start_at.elapsed().as_millis()
    );

    Ok(())
}

#[allow(unused)]
async fn call2(times: usize, _guard: WaitGroupGuard<'_>) -> Result<()> {
    let client = ClientBuilder::new(config::ECHO_SERVER_IP, config::ECHO_SERVER_PORT).connect()?;
    let writer = client.writer();
    let mut reader = client.reader();
    let mut size = 0usize;
    for _ in 0..100 {
        writer
            .lock()
            .await
            .write("hello, echo server".as_bytes())
            .await?
            .write(config::CRLF.as_bytes())
            .await?
            .flush()
            .await?;
        size += reader.read_until_exclusive(config::CRLF).await?.len();
    }
    // size += reader.read_once().await?.len();
    log::info!("total get response: {}", size);
    Ok(())
}
