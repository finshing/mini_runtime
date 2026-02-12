use mini_redis::{config, request_handler, result::RedisResult};
use mini_runtime::create_server;

#[rt_entry::main(log_level = "debug")]
async fn main() -> RedisResult<()> {
    let mut server = create_server!(
        config::REDIS_SERVER_IP,
        config::REDIS_SERVER_PORT,
        request_handler
    )?;

    server.run().await?;

    Ok(())
}
