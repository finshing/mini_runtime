use std::time;

use mini_runtime::{
    config::{self, CRLF},
    create_server,
    io_ext::{read::AsyncReader, write::AsyncBufWriter},
    result::Result,
    sleep,
    web::conn::Conn,
};

#[rt_entry::main(log_level = "info")]
async fn main() -> Result<()> {
    let mut server = create_server!(config::ECHO_SERVER_IP, config::ECHO_SERVER_PORT, handler)?;
    server
        .update_timeout(|timeout| {
            timeout
                .update_timeout(time::Duration::from_secs(10))
                .set_read_timeout(time::Duration::from_secs(5));
        })
        .set_max_wait_time(time::Duration::from_secs(5))
        .run()
        .await?;
    Ok(())
}

async fn handler(conn: Conn) -> Result<()> {
    let mut reader = AsyncReader::from(conn.clone());
    let buf_writer = AsyncBufWriter::from(conn.clone());
    let body = reader.read_until_exclusive(CRLF).await?;

    sleep(time::Duration::from_millis(500)).await;

    buf_writer
        .lock()
        .await
        .write(&body)
        .await?
        .write(CRLF.as_bytes())
        .await?
        .flush()
        .await?;
    Ok(())
}
