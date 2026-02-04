use std::time;

use mini_runtime::{
    config,
    result::Result,
    sleep,
    sync::wait_group::{WaitGroup, WaitGroupGuard},
    web::client::Client,
};

#[rt_entry::main]
async fn main() -> Result<()> {
    let start_at = time::Instant::now();
    let wg = WaitGroup::new();
    for i in 0..10i64 {
        spawn!(call((i - 5).pow(2) * 10, wg.add()));
    }

    wg.wait().await;
    log::info!("total cost {}ms", start_at.elapsed().as_millis());
    Ok(())
}

async fn call(dur: i64, _guard: WaitGroupGuard<'_>) -> Result<()> {
    let start_at = time::Instant::now();
    let client = Client::connect(config::ECHO_SERVER_IP, config::ECHO_SERVER_PORT)?;
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

// async fn call2(times: usize, _guard: WaitGroupGuard<'_>) -> Result<()> {
//     let client = Client::connect(config::ECHO_SERVER_IP, config::ECHO_SERVER_PORT)?;
//     for _ in 0..times {
//         client.writer().s
//     }
// }
