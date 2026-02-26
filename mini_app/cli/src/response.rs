use common::{
    CRLFCRLF, CT_EVENT_STREAM, HttpHeader, HttpProtocol, HttpStatus, LFLF, TE_CHUNKED,
    helper::{BytesSplitter, slice_to_str},
    http_reader::{
        ChunkedBodyReader, FixedLengthBodyReader, THttpBodyReader, parse_http_header,
        read_json_from_vec,
    },
    result::HttpResult,
    sse_proto::SSEProto,
};
use memchr::memmem;
use mini_runtime::{
    config::CRLF,
    io_ext::read::{AsyncReader, TAsyncBufRead},
    take_vec_at,
};
use serde::de::DeserializeOwned;

pub struct ClientResponse<R: TAsyncBufRead + 'static> {
    protocol: HttpProtocol,
    status: HttpStatus,
    header: HttpHeader,

    reader: AsyncReader<R>,
}

impl<R: TAsyncBufRead + 'static> ClientResponse<R> {
    pub async fn new(mut reader: AsyncReader<R>) -> HttpResult<Self> {
        let (protocol, status) = Self::parse_first_line(reader.read_until_exclusive(CRLF).await?)?;
        let header = parse_http_header(reader.read_until_exclusive(CRLFCRLF).await?)?;
        log::info!("http header: {:?}", header);

        Ok(Self {
            protocol,
            status,
            header,

            reader,
        })
    }

    pub fn protocol(&self) -> &HttpProtocol {
        &self.protocol
    }

    pub fn status(&self) -> &HttpStatus {
        &self.status
    }

    pub fn header(&self) -> &HttpHeader {
        &self.header
    }

    fn parse_first_line(data: Vec<u8>) -> HttpResult<(HttpProtocol, HttpStatus)> {
        log::debug!("first line: {}", String::from_utf8_lossy(data.as_slice()));
        let mut splitter = BytesSplitter::new(data.as_slice(), " ".as_bytes()).map(slice_to_str);
        let protocol = HttpProtocol::new(splitter.next().ok_or("protocol not exist")??)?;
        let status = splitter
            .next()
            .ok_or("status not exist")??
            .parse::<usize>()?
            .into();

        Ok((protocol, status))
    }

    pub fn body_reader(&self) -> Box<dyn THttpBodyReader> {
        if self.header().get_content_type() == Some(CT_EVENT_STREAM) {
            Box::new(self.sse_reader().unwrap())
        } else if self.header.get_transfer_encoding() == Some(TE_CHUNKED) {
            Box::new(ChunkedBodyReader::new(self.reader.clone()))
        } else {
            let content_length = self.header.content_length().unwrap_or(0);
            Box::new(FixedLengthBodyReader::new(
                self.reader.clone(),
                content_length,
            ))
        }
    }

    pub fn sse_reader(&self) -> Option<SSEBodyReader<R>> {
        if self.header.get_content_type() == Some(CT_EVENT_STREAM) {
            let reader = ChunkedBodyReader::new(self.reader.clone());
            Some(SSEBodyReader::new(reader))
        } else {
            None
        }
    }

    pub async fn json<T: DeserializeOwned>(&mut self) -> HttpResult<T> {
        let mut reader = self.body_reader();
        let body = reader.read().await?;
        read_json_from_vec(body)
    }
}

pub struct SSEBodyReader<R: TAsyncBufRead> {
    reader: ChunkedBodyReader<R>,
    buf: Vec<u8>,
}

impl<R: TAsyncBufRead> SSEBodyReader<R> {
    pub fn new(reader: ChunkedBodyReader<R>) -> Self {
        Self {
            reader,
            buf: Vec::new(),
        }
    }

    pub async fn read_event(&mut self) -> HttpResult<Option<SSEProto>> {
        let event_body = self.read().await?;
        if event_body.is_empty() {
            return Ok(None);
        }
        let event = SSEProto::deserialize(slice_to_str(&event_body)?)?;
        Ok(Some(event))
    }
}

impl<R: TAsyncBufRead> THttpBodyReader for SSEBodyReader<R> {
    fn read(&mut self) -> common::HttpBoxedFuture<'_, Vec<u8>> {
        Box::pin(async {
            loop {
                if let Some(offset) = memmem::find(&self.buf, LFLF.as_bytes()) {
                    // 需要将结尾的\n\n也取出来，避免下次read时的二次读取（关键）
                    let mut event_body = take_vec_at(&mut self.buf, offset + 2);
                    // 删除掉最后的\n\n
                    unsafe {
                        event_body.set_len(offset);
                    }
                    return Ok(event_body);
                }
                match self.reader.read().await {
                    Ok(data) => self.buf.extend(data),
                    Err(e) if e.is_eof() => {
                        return Ok(Vec::new());
                    }
                    Err(e) => return Err(e),
                }
            }
        })
    }
}
