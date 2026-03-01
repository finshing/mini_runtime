use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc};

use common::{
    CT_TEXT_PLAIN, HttpBoxedFuture, HttpHeader, HttpMethod, HttpProtocol, TE_CHUNKED, Url,
    http_writer::HttpBodyWriter,
    result::{HttpError, HttpResult},
};

#[cfg(feature = "mock")]
use common::{
    CRLFCRLF,
    helper::{BytesSplitter, slice_to_str},
};

use mini_runtime::{
    ConnTimeout,
    io_ext::{
        read::{AsyncReader, TAsyncBufRead, TAsyncRead},
        write::{AsyncBufWriter, TAsyncWrite},
    },
    result::ErrorType,
    sync::mutex::AsyncMutex,
    web::{client::ClientBuilder, conn::TcpConn},
};
use serde::Serialize;

use crate::response::ClientResponse;

#[cfg(feature = "tcp")]
pub fn new_client() -> HttpClient<TcpConn> {
    HttpClient::<TcpConn>::default()
}

#[cfg(feature = "mock")]
pub fn new_client() -> HttpClient<MockTcpConn> {
    HttpClient::<MockTcpConn>::default()
}

trait HttpConn<C: TAsyncBufRead + TAsyncWrite + 'static> {
    fn conn(
        &mut self,
        domain: String,
        port: u16,
    ) -> HttpBoxedFuture<'_, (AsyncBufWriter<C>, AsyncReader<C>)>;
}

pub struct HttpClient<C: TAsyncBufRead + TAsyncWrite + 'static> {
    method: HttpMethod,
    url: Option<Url>,
    protocol: HttpProtocol,
    header: HttpHeader,
    timeout: ConnTimeout,

    _mock_buf: Rc<RefCell<Vec<u8>>>,
    _mark: PhantomData<C>,
}

const DEFAULT_USER_AGENT: &str = "mini client/1.0";

impl<C: TAsyncBufRead + TAsyncWrite + 'static> HttpClient<C> {
    pub fn set_url(&mut self, url_str: impl AsRef<str>) -> HttpResult<&mut Self> {
        self.url.replace(Url::from(url_str)?);
        Ok(self)
    }

    pub fn set_protocol(&mut self, protocol: HttpProtocol) -> &mut Self {
        self.protocol = protocol;
        self
    }

    pub fn update_header(&mut self, mut updater: impl FnMut(&mut HttpHeader)) -> &mut Self {
        updater(&mut self.header);
        self
    }

    pub fn update_timeout(&mut self, mut updater: impl FnMut(&mut ConnTimeout)) -> &mut Self {
        updater(&mut self.timeout);
        self
    }

    pub async fn get(
        &mut self,
        params: Option<HashMap<String, String>>,
    ) -> HttpResult<ClientResponse<C>> {
        if let Some(params) = params
            && let Some(url) = &mut self.url
        {
            url.path.set_params(params);
        }

        self.send(HttpMethod::Get, &[]).await
    }

    pub async fn post(&mut self, body: &[u8]) -> HttpResult<ClientResponse<C>> {
        self.send(HttpMethod::Post, body).await
    }

    pub async fn post_json<T: Serialize>(&mut self, value: &T) -> HttpResult<ClientResponse<C>> {
        self.post(serde_json::to_string(value)?.as_bytes()).await
    }

    // 发送chunked请求
    pub async fn post_chunk(&mut self) -> HttpResult<ChunkedTransport<C, C>> {
        let (writer, reader) = self.connect(HttpMethod::Post).await?;

        self.header.set_transfer_encoding(TE_CHUNKED.into());
        let mut body_writer = HttpBodyWriter::new(writer.clone());
        body_writer.write_first_line(&self.first_line()?)?;
        body_writer.write_header(&self.header)?;
        body_writer.flush().await?;

        Ok(ChunkedTransport { writer, reader })
    }

    async fn send(&mut self, method: HttpMethod, body: &[u8]) -> HttpResult<ClientResponse<C>> {
        let (writer, reader) = self.connect(method).await?;
        self.header
            .set_content_length(body.len().to_string().into())
            .clear_transfer_encoding();
        if self.header.get_content_type().is_none() {
            self.header.set_content_type(CT_TEXT_PLAIN.into());
        }

        let mut body_writer = HttpBodyWriter::new(writer);
        body_writer.write_first_line(&self.first_line()?)?;
        body_writer.write_header(&self.header)?;
        body_writer.write_fix_length_body(body).await?;

        ClientResponse::new(reader).await
    }

    async fn connect(
        &mut self,
        method: HttpMethod,
    ) -> HttpResult<(AsyncBufWriter<C>, AsyncReader<C>)> {
        self.method = method;
        let url = self
            .url
            .as_ref()
            .ok_or(HttpError::InvalidUrl(common::result::InvalidUrl::Domain))?;
        if self.header.get_user_agent().is_none() {
            self.header.set_user_agent(DEFAULT_USER_AGENT.into());
        }
        self.header.set_host((&url.domain.host).into());

        self.conn(url.domain.host.to_owned(), url.domain.port.unwrap_or(80))
            .await
    }

    fn first_line(&self) -> HttpResult<String> {
        if let Some(url) = &self.url {
            Ok(format!("{} {} {}", self.method, url.path, self.protocol))
        } else {
            Err(common::result::HttpError::InvalidUrl(
                common::result::InvalidUrl::Path,
            ))
        }
    }
}

impl<C: TAsyncBufRead + TAsyncWrite + 'static> Default for HttpClient<C> {
    fn default() -> Self {
        Self {
            method: HttpMethod::Get,
            url: None,
            protocol: HttpProtocol::default(),
            header: HttpHeader::default(),
            timeout: ConnTimeout::new(None),
            _mock_buf: Rc::new(RefCell::new(Vec::new())),
            _mark: PhantomData,
        }
    }
}

#[cfg(feature = "mock")]
impl HttpClient<MockTcpConn> {
    pub fn request_header(&self) -> Option<String> {
        let buf = self._mock_buf.borrow();
        let mut splitter = BytesSplitter::new(&buf, CRLFCRLF.as_bytes());
        splitter
            .next()
            .map(slice_to_str)
            .map(|s| s.map(|s| s.to_owned()).unwrap_or_default())
    }

    pub fn request_body(&self) -> Option<Vec<u8>> {
        let buf = self._mock_buf.borrow();
        let mut splitter = BytesSplitter::new(&buf, CRLFCRLF.as_bytes());
        splitter.next();
        splitter.next().map(|b| b.to_owned())
    }
}

impl<C: TAsyncBufRead + TAsyncWrite + 'static> HttpConn<C> for HttpClient<C> {
    default fn conn(
        &mut self,
        _domain: String,
        _port: u16,
    ) -> HttpBoxedFuture<'_, (AsyncBufWriter<C>, AsyncReader<C>)> {
        Box::pin(async { Err(HttpError::Closed) })
    }
}

impl HttpConn<TcpConn> for HttpClient<TcpConn> {
    fn conn(
        &mut self,
        domain: String,
        port: u16,
    ) -> HttpBoxedFuture<'_, (AsyncBufWriter<TcpConn>, AsyncReader<TcpConn>)> {
        Box::pin(async move {
            let client = ClientBuilder::new(&domain, port)
                .update_timeout(|conn_timeout| {
                    *conn_timeout = self.timeout.clone();
                })
                .connect()
                .await?;

            Ok((client.writer(), client.reader()))
        })
    }
}

impl HttpConn<MockTcpConn> for HttpClient<MockTcpConn> {
    fn conn(
        &mut self,
        _domain: String,
        _port: u16,
    ) -> HttpBoxedFuture<'_, (AsyncBufWriter<MockTcpConn>, AsyncReader<MockTcpConn>)> {
        Box::pin(async {
            let mock_conn = Rc::new(AsyncMutex::new(MockTcpConn {
                buf: self._mock_buf.clone(),
            }));
            Ok((mock_conn.clone().into(), mock_conn.into()))
        })
    }
}

// 对Tcp连接的Mock，不真正创建连接，方便对请求body进行debug
pub struct MockTcpConn {
    buf: Rc<RefCell<Vec<u8>>>,
}

impl TAsyncWrite for MockTcpConn {
    fn ready_to_write(&mut self) -> mini_runtime::BoxedFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn write(&mut self, data: &[u8]) -> mini_runtime::result::Result<usize> {
        self.buf.borrow_mut().extend_from_slice(data);
        Ok(data.len())
    }
}

impl TAsyncRead for MockTcpConn {
    fn ready_to_read(&mut self) -> mini_runtime::BoxedFuture<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn read(&mut self, _buf: &mut [u8]) -> mini_runtime::result::Result<usize> {
        Err(ErrorType::Eof.into())
    }
}

impl TAsyncBufRead for MockTcpConn {
    fn read_util<'a>(
        &'a mut self,
        _read_while: impl Fn(&[u8]) -> Option<usize> + 'a,
    ) -> mini_runtime::BoxedFuture<'a, Vec<u8>> {
        Box::pin(async { Err(ErrorType::Eof.into()) })
    }
}

pub struct ChunkedTransport<R: TAsyncBufRead + 'static, W: TAsyncWrite> {
    writer: AsyncBufWriter<W>,
    reader: AsyncReader<R>,
}

impl<R: TAsyncBufRead + 'static, W: TAsyncWrite> ChunkedTransport<R, W> {
    pub async fn send_chunk(&mut self, body: &[u8]) -> HttpResult<()> {
        let mut body_writer = HttpBodyWriter::new(self.writer.clone());
        body_writer.write_chunked_body(body).await
    }

    pub async fn wait_resp(&mut self) -> HttpResult<ClientResponse<R>> {
        self.send_chunk(&[]).await?;
        ClientResponse::new(self.reader.clone()).await
    }
}
