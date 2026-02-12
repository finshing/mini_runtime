use mini_redis::{client::RedisClient, config, request::Request, result::RedisResult};

#[rt_entry::main(log_level = "info")]
async fn main() -> RedisResult<()> {
    client_run().await
}

async fn client_run() -> RedisResult<()> {
    let client = RedisClient::new(config::REDIS_SERVER_IP, config::REDIS_SERVER_PORT)?;

    query(&client, Request::Get("rust".to_owned())).await?;
    query(
        &client,
        Request::Set("rust".to_owned(), "v1.93.0".to_owned()),
    )
    .await?;
    query(&client, Request::Get("rust".to_owned())).await?;
    query(&client, Request::Del("rust".to_owned())).await?;
    query(&client, Request::Get("rust".to_owned())).await?;

    Ok(())
}

async fn query(client: &RedisClient, cmd: Request) -> RedisResult<()> {
    let response = client.cmd(&cmd).await?;
    log::info!("cmd {:?} with response {:?}", cmd, response);
    Ok(())
}
