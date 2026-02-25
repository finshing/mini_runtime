use mini_runtime::{
    io_ext::read::{AsyncReader, TAsyncBufRead},
    variable_log,
};
use serde::de::DeserializeOwned;

use crate::{
    CRLF, HttpBoxedFuture, HttpHeader,
    helper::{BytesSplitter, slice_to_str},
    result::{HttpError, HttpResult, InvalidBody},
};

pub fn parse_http_header(data: Vec<u8>) -> HttpResult<HttpHeader> {
    log::debug!("headers: {}", String::from_utf8_lossy(data.as_slice()));
    fn split_key_value(line: &[u8]) -> HttpResult<(String, String)> {
        let mut splitter = BytesSplitter::new(line, ":".as_bytes()).map(slice_to_str);
        let key = splitter.next().ok_or("empty header key")??.to_owned();
        let value = splitter
            .next()
            .ok_or("empty header value")??
            .strip_prefix(' ')
            .map(ToOwned::to_owned)
            .unwrap_or_default();

        Ok((key, value))
    }

    let mut headers = HttpHeader::default();
    let lines = BytesSplitter::new(data.as_slice(), CRLF.as_bytes()).collect::<Vec<_>>();
    for line in lines {
        let (key, value) = split_key_value(line)?;
        headers.set(key.into(), value.into());
    }

    Ok(headers)
}

#[inline]
pub fn read_json_from_vec<T: DeserializeOwned>(body: Vec<u8>) -> HttpResult<T> {
    Ok(serde_json::from_slice(body.as_slice())?)
}

pub trait THttpBodyReader {
    fn read(&mut self) -> HttpBoxedFuture<'_, Vec<u8>>;
}

pub struct FixedLengthBodyReader<R: TAsyncBufRead> {
    reader: AsyncReader<R>,
    content_length: usize,
}

impl<R: TAsyncBufRead> FixedLengthBodyReader<R> {
    pub fn new(reader: AsyncReader<R>, content_length: usize) -> Self {
        Self {
            reader,
            content_length,
        }
    }
}

impl<R: TAsyncBufRead> THttpBodyReader for FixedLengthBodyReader<R> {
    fn read(&mut self) -> HttpBoxedFuture<'_, Vec<u8>> {
        Box::pin(async {
            if self.content_length == 0 {
                return Ok(Vec::new());
            }

            Ok(self.reader.read_exactly(self.content_length).await?)
        })
    }
}

pub struct ChunkedBodyReader<R: TAsyncBufRead> {
    reader: AsyncReader<R>,
}

impl<R: TAsyncBufRead> ChunkedBodyReader<R> {
    pub fn new(reader: AsyncReader<R>) -> Self {
        Self { reader }
    }

    async fn read_chunk_length(&mut self) -> HttpResult<usize> {
        let body = self.reader.read_until_exclusive(CRLF).await?;
        // 按十六进制字符串解析
        let length = usize::from_str_radix(str::from_utf8(body.as_slice())?, 16)?;
        Ok(length)
    }

    async fn read_chunk(&mut self, chunk_length: usize) -> HttpResult<Vec<u8>> {
        let mut body = Vec::new();
        if chunk_length != 0 {
            body = self.reader.read_exactly(chunk_length).await?;
        }
        // 根据协议要求，body需要以CRLF结尾，因此这里需要完成分隔符的读取
        let crlf = self.reader.read_until(CRLF).await?;
        if chunk_length == 0 {
            Err(HttpError::Eof)
        } else if crlf.len() != CRLF.len() {
            log::warn!("Invalid chunked body. Not end with CRLF, with {:?}", crlf);
            Err(HttpError::InvalidBody(InvalidBody::ChunkBodyRead))
        } else {
            Ok(body)
        }
    }
}

impl<R: TAsyncBufRead> THttpBodyReader for ChunkedBodyReader<R> {
    fn read(&mut self) -> HttpBoxedFuture<'_, Vec<u8>> {
        Box::pin(async {
            let chunk_length =
                variable_log!(debug @ self.read_chunk_length().await, "[read_chunk_length]")?;
            self.read_chunk(chunk_length).await
        })
    }
}
