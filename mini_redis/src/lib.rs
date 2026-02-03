use std::time;

use mini_runtime::{
    err_log,
    io_ext::{
        read::{AsyncReader, TAsyncBufRead},
        write::{AsyncWriter, TAsyncWrite},
    },
    variable_log,
    web::conn::Conn,
};

use crate::{db::db_op, request::Request, response::Response, result::RedisResult};

pub mod client;
pub mod config;
pub(crate) mod db;
pub mod request;
pub mod response;
pub mod result;

const CRLF: &str = "\r\n";

pub(crate) async fn send<W: TAsyncWrite>(writer: AsyncWriter<W>, data: &[u8]) -> RedisResult<()> {
    let mut writer = writer.lock().await;
    writer.write(data).await?;
    writer.write(CRLF.as_bytes()).await?;
    Ok(())
}

pub(crate) async fn receive<R: TAsyncBufRead>(mut reader: AsyncReader<R>) -> RedisResult<Vec<u8>> {
    Ok(reader.read_until_exclusive(CRLF).await?)
}

// 长连接
pub async fn request_handler(conn: Conn) -> RedisResult<()> {
    let handler = Handler { conn };

    loop {
        let start_at = time::Instant::now();
        let req = variable_log!(debug @ handler.receive().await, "[redis request]")?;
        log::info!(
            "{:?} op receive cost {}ms",
            req,
            start_at.elapsed().as_millis()
        );
        let resp = variable_log!(debug @ db_op(&req), "[redis response]");

        err_log!(handler.send(resp).await)?;
        log::info!("{:?} total cost {}ms", req, start_at.elapsed().as_millis());
    }
}

struct Handler {
    conn: Conn,
}

impl Handler {
    async fn receive(&self) -> RedisResult<Request> {
        let body = receive(self.conn.clone().into()).await?;
        Ok(serde_json::from_slice::<Request>(&body)?)
    }

    async fn send(&self, resp: Response) -> RedisResult<()> {
        let resp_body = serde_json::to_string(&resp)?;
        send(self.conn.clone().into(), resp_body.as_bytes()).await
    }
}
