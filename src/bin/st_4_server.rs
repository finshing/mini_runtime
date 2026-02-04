use mini_runtime::{
    config, create_server,
    io_ext::{read::AsyncReader, write::AsyncBufWriter},
    result::Result,
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
    let mut reader = AsyncReader::from(conn.clone());
    let buf_writer = AsyncBufWriter::from(conn.clone());
    loop {
        let result = reader.read_once().await?;
        if result.is_empty() {
            break Ok(());
        }

        buf_writer
            .lock()
            .await
            .write(&result)
            .await?
            .write(format!("(size={})", result.len()).as_bytes())
            .await?
            .flush()
            .await?;
    }
}
