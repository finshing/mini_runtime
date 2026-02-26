use std::{fmt::Write, rc::Rc};

use common::{
    CT_APPLICATION_JSON, CT_TEXT_HTML, CT_TEXT_PLAIN, HttpHeader, HttpProtocol, HttpStatus,
    TE_CHUNKED, http_writer::HttpBodyWriter, result::HttpResult, sse_proto::SSEProto,
};
use mini_runtime::{
    err_log,
    io_ext::write::{AsyncBufWriter, TAsyncWrite},
    sync::mutex::AsyncMutex,
    web::conn::{SharedTcpConn, TcpConn},
};
use serde::ser::Serialize;

use crate::helper::load_file;

pub type ServerResponse = Rc<AsyncMutex<_ServerResponse<TcpConn>>>;

pub fn create_response(conn: SharedTcpConn, protocol: HttpProtocol) -> ServerResponse {
    Rc::new(AsyncMutex::new(_ServerResponse::new(conn.into(), protocol)))
}

pub struct _ServerResponse<W: TAsyncWrite> {
    protocol: HttpProtocol,
    status: HttpStatus,
    header: HttpHeader,
    writer: AsyncBufWriter<W>,
}

impl<W: TAsyncWrite> _ServerResponse<W> {
    pub fn new(writer: AsyncBufWriter<W>, protocol: HttpProtocol) -> Self {
        Self {
            protocol,
            status: HttpStatus::Success,
            header: HttpHeader::default(),
            writer,
        }
    }

    pub fn set_status(&mut self, status: HttpStatus) -> &mut Self {
        self.status = status;
        self
    }

    pub fn update_header(&mut self, mut updater: impl FnMut(&mut HttpHeader)) -> &mut Self {
        updater(&mut self.header);
        self
    }

    pub async fn json<T: Serialize>(&mut self, data: &T) -> HttpResult<()> {
        let body = serde_json::to_string(data)?;
        self.update_header(|header| {
            header.set_content_type(CT_APPLICATION_JSON.into());
        })
        .send(body.as_bytes())
        .await
    }

    pub async fn html_file(&mut self, path: &str) -> HttpResult<()> {
        self.html(&load_file(path)?).await
    }

    pub async fn html(&mut self, data: &str) -> HttpResult<()> {
        self.update_header(|header| {
            header.set_content_type(CT_TEXT_HTML.into());
        })
        .send(data.as_bytes())
        .await
    }

    pub async fn send(&mut self, body: &[u8]) -> HttpResult<()> {
        self.header
            .set_content_length(body.len().to_string().into());
        if self.header.get_content_type().is_none() {
            self.header.set_content_type(CT_TEXT_PLAIN.into());
        }

        let mut body_writer = HttpBodyWriter::new(self.writer.clone());
        body_writer.write_first_line(&self.first_line())?;
        body_writer.write_header(&self.header)?;
        body_writer.write_fix_length_body(body).await
    }

    pub async fn chunk(&mut self) -> HttpResult<ChunkedResponse<W>> {
        self.header
            .set_transfer_encoding(TE_CHUNKED.into())
            .clear_content_length();

        let mut body_writer = HttpBodyWriter::new(self.writer.clone());
        body_writer.write_first_line(&self.first_line())?;
        body_writer.write_header(&self.header)?;
        body_writer.flush().await?;

        Ok(ChunkedResponse { body_writer })
    }

    fn first_line(&self) -> String {
        format!(
            "{} {} {}",
            self.protocol,
            self.status.code(),
            self.status.description()
        )
    }
}

pub struct ChunkedResponse<W: TAsyncWrite> {
    body_writer: HttpBodyWriter<W>,
}

impl<W: TAsyncWrite> ChunkedResponse<W> {
    pub async fn write_chunk(&mut self, data: &[u8]) -> HttpResult<()> {
        self.body_writer.write_chunked_body(data).await
    }

    pub async fn close(&mut self) -> HttpResult<()> {
        self.write_chunk(&[]).await
    }
}

/// 通过write_event()方法将事件写入缓冲中，
/// 然后通过flush()方法刷新缓冲区，
/// 最后需要通过close方法来关闭响应
/// let mut sse_response = chunk_response.into();
/// sse_response
///     .write_event(SSEProto::default)
///     .write_event(SSEProto::default)
///     .flush()
///     .await?
///     .write_event(SSEProto::default)
///     .close()
///     .await?;
pub struct SSEResponse<W: TAsyncWrite> {
    response: ChunkedResponse<W>,
    events: Vec<SSEProto>,
}

impl<W: TAsyncWrite> SSEResponse<W> {
    pub fn write_event(&mut self, event: SSEProto) -> &mut Self {
        self.events.push(event);
        self
    }

    pub async fn flush(&mut self) -> HttpResult<&mut Self> {
        if !self.events.is_empty() {
            let mut data = String::new();
            for event in self.events.drain(..) {
                let _ = err_log!(
                    data.write_fmt(format_args!("{}", event)),
                    "sse response failed"
                );
            }
            self.response.write_chunk(data.as_bytes()).await?;
        }
        Ok(self)
    }

    pub async fn close(&mut self) -> HttpResult<()> {
        self.flush().await?;
        self.response.close().await
    }
}

impl<W: TAsyncWrite> From<ChunkedResponse<W>> for SSEResponse<W> {
    fn from(response: ChunkedResponse<W>) -> Self {
        Self {
            response,
            events: Vec::new(),
        }
    }
}
