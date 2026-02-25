use common::{
    CRLF, CRLFCRLF, HttpHeader, HttpMethod, HttpProtocol, TE_CHUNKED, UrlPath,
    helper::{BytesSplitter, slice_to_str},
    http_reader::{
        ChunkedBodyReader, FixedLengthBodyReader, THttpBodyReader, parse_http_header,
        read_json_from_vec,
    },
    result::HttpResult,
};
use mini_runtime::{
    io_ext::read::{AsyncReader, TAsyncBufRead},
    web::conn::_Conn,
};
use serde::de::DeserializeOwned;

pub type ServerRequest = _ServerRequest<_Conn>;

pub struct _ServerRequest<R: TAsyncBufRead + 'static> {
    method: HttpMethod,
    url: UrlPath,
    protocol: HttpProtocol,
    header: HttpHeader,

    reader: AsyncReader<R>,
}

impl<R: TAsyncBufRead + 'static> _ServerRequest<R> {
    pub async fn new(mut reader: AsyncReader<R>) -> HttpResult<Self> {
        let (method, url, protocol) =
            Self::parse_first_line(reader.read_until_exclusive(CRLF).await?)?;
        let header = parse_http_header(reader.read_until_exclusive(CRLFCRLF).await?)?;

        Ok(Self {
            method,
            url,
            protocol,
            header,
            reader,
        })
    }

    pub fn method(&self) -> &HttpMethod {
        &self.method
    }

    pub fn protocol(&self) -> &HttpProtocol {
        &self.protocol
    }

    pub fn url(&self) -> &UrlPath {
        &self.url
    }

    pub fn header(&self) -> &HttpHeader {
        &self.header
    }

    fn parse_first_line(data: Vec<u8>) -> HttpResult<(HttpMethod, UrlPath, HttpProtocol)> {
        log::debug!("first line: {}", String::from_utf8_lossy(data.as_slice()));
        let mut splitter = BytesSplitter::new(data.as_slice(), " ".as_bytes()).map(slice_to_str);
        let method = splitter.next().ok_or("method not exist")??.into();
        let url = splitter.next().ok_or("url not exist")??.into();
        let protocol = HttpProtocol::new(splitter.next().ok_or("protocol not exist")??)?;

        Ok((method, url, protocol))
    }

    pub fn body_reader(&self) -> Box<dyn THttpBodyReader> {
        if self.header.get_transfer_encoding() == Some(TE_CHUNKED) {
            Box::new(ChunkedBodyReader::new(self.reader.clone()))
        } else {
            let content_length = self.header.content_length().unwrap_or(0);
            Box::new(FixedLengthBodyReader::new(
                self.reader.clone(),
                content_length,
            ))
        }
    }

    pub async fn json<T: DeserializeOwned>(&self) -> HttpResult<T> {
        let mut reader = self.body_reader();
        let body = reader.read().await?;
        if body.is_empty() {
            return Err(common::result::HttpError::Eof);
        }
        read_json_from_vec(body)
    }
}
