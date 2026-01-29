use core::time;

use mini_runtime::{
    config, create_server,
    io_ext::{read::AsyncReader, write::AsyncWriter},
    result::Result,
    sleep,
    web::conn::Conn,
};

#[rt_entry::main]
async fn main() -> Result<()> {
    let mut server = create_server!(
        config::ECHO_SERVER_IP,
        config::ECHO_SERVER_PORT,
        echo_server_handler,
    )?;

    server.run().await?;
    Ok(())
}

async fn echo_server_handler(conn: Conn) -> Result<()> {
    let result = AsyncReader::from(conn.clone()).read_once().await?;

    AsyncWriter::from(conn.clone())
        .lock()
        .await
        .write(result.as_ref())
        .await?;

    sleep(time::Duration::from_millis(100)).await;
    AsyncWriter::from(conn.clone())
        .lock()
        .await
        .write(format!("(size={})", result.len()).as_ref())
        .await?;

    Ok(())
}
