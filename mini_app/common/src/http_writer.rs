use mini_runtime::io_ext::write::{AsyncBufWriter, TAsyncWrite};

use crate::{CRLF, HttpHeader, allow_body_write, result::HttpResult};

#[derive(Default)]
struct BufWriter {
    buf: Vec<u8>,
}

impl BufWriter {
    fn write(&mut self, data: &[u8]) -> &mut Self {
        self.buf.extend_from_slice(data);
        self
    }

    fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buf)
    }

    fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }
}

#[derive(Clone, Copy, Debug)]
enum HttpBodyWriteState {
    FirstLineWriting,
    HeaderWriting,
    BodyWriting,
    ChunkedBodyWriting,
    Closed,
}

/// 先将数据写入缓冲区中，只有通过flush才能正式将数据发送给socket
pub struct HttpBodyWriter<W: TAsyncWrite> {
    writer: AsyncBufWriter<W>,
    buf: BufWriter,
    state: HttpBodyWriteState,
}

impl<W: TAsyncWrite> HttpBodyWriter<W> {
    pub fn new(writer: AsyncBufWriter<W>) -> Self {
        Self {
            writer,
            buf: BufWriter::default(),
            state: HttpBodyWriteState::FirstLineWriting,
        }
    }

    pub fn write_first_line(&mut self, first_line: &str) -> HttpResult<()> {
        allow_body_write!(
            self.state,
            HttpBodyWriteState::FirstLineWriting,
            "first line writing"
        );

        self.write_with_boundary(first_line.as_bytes());
        self.state = HttpBodyWriteState::HeaderWriting;
        Ok(())
    }

    pub fn write_header(&mut self, header: &HttpHeader) -> HttpResult<()> {
        allow_body_write!(
            self.state,
            HttpBodyWriteState::HeaderWriting,
            "header writing"
        );

        for (key, value) in header.iter() {
            self.buf
                .write(key.as_bytes())
                .write(": ".as_bytes())
                .write(value.as_bytes());
            self.write_with_boundary(&[]);
        }

        // header的结束行
        self.write_with_boundary(&[]);
        self.state = HttpBodyWriteState::BodyWriting;
        Ok(())
    }

    pub async fn write_fix_length_body(&mut self, data: &[u8]) -> HttpResult<()> {
        allow_body_write!(self.state, HttpBodyWriteState::BodyWriting, "body writing");

        self.write_with_boundary(data);
        self.flush().await?;
        self.state = HttpBodyWriteState::Closed;
        Ok(())
    }

    pub async fn write_chunked_body(&mut self, data: &[u8]) -> HttpResult<()> {
        if matches!(self.state, HttpBodyWriteState::BodyWriting) {
            self.state = HttpBodyWriteState::ChunkedBodyWriting;
        }
        allow_body_write!(
            self.state,
            HttpBodyWriteState::ChunkedBodyWriting,
            "chunked body writing"
        );

        // 将chunk的长度转换为16进制的字符串
        self.write_with_boundary(format!("{:x}", data.len()).as_bytes());
        self.write_with_boundary(data);
        self.flush().await?;
        if data.is_empty() {
            self.state = HttpBodyWriteState::Closed;
        }

        Ok(())
    }

    pub async fn flush(&mut self) -> HttpResult<()> {
        if self.buf.is_empty() {
            return Ok(());
        }

        let mut writer = self.writer.lock().await;
        writer
            .write(self.buf.take().as_slice())
            .await?
            .flush()
            .await?;
        Ok(())
    }

    #[inline]
    fn write_with_boundary(&mut self, data: &[u8]) {
        self.buf.write(data);
        self.buf.write(CRLF.as_bytes());
    }
}
