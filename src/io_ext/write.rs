use std::rc::Rc;

use crate::{
    BoxedFuture,
    config::MAX_WRITE_BUF_SIZE,
    err_log,
    helper::take_vec_at,
    result::Result,
    sync::mutex::{AsyncMutex, AsyncMutexGuard},
};

pub trait TAsyncWrite {
    // 异步判断是否可写
    fn ready_to_write(&mut self) -> BoxedFuture<'_, ()>;

    fn write(&mut self, data: &[u8]) -> Result<usize>;
}

#[derive(Clone)]
pub struct AsyncBufWriter<W: TAsyncWrite> {
    writer: Rc<AsyncMutex<W>>,
}

impl<W: TAsyncWrite> AsyncBufWriter<W> {
    fn new(writer: Rc<AsyncMutex<W>>) -> Self {
        Self { writer }
    }

    pub async fn lock(&self) -> _AsyncBufWriterGuard<'_, W> {
        _AsyncBufWriterGuard::new(self).await
    }
}

impl<W: TAsyncWrite> From<Rc<AsyncMutex<W>>> for AsyncBufWriter<W> {
    fn from(writer: Rc<AsyncMutex<W>>) -> Self {
        Self::new(writer)
    }
}

pub struct _AsyncBufWriterGuard<'a, W: TAsyncWrite> {
    writer: AsyncMutexGuard<'a, W>,
    buf: Vec<u8>,
}

impl<'a, W: TAsyncWrite> _AsyncBufWriterGuard<'a, W> {
    async fn new(buf_writer: &'a AsyncBufWriter<W>) -> Self {
        let writer = buf_writer.writer.lock().await;
        Self {
            writer,
            buf: Vec::new(),
        }
    }

    // write + flush
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        self.write(data).await?.flush().await
    }

    /// write用于写入应用层缓冲区，并在缓冲区满的时候通过flush写入内核
    /// 可以如此调用：
    /// let writer = AsycnBufWriter::new(writer)
    ///     .lock()
    ///     .await
    ///     .write("hello".as_bytes())
    ///     .await?
    ///     .write(", mini_runtime".as_bytes())
    ///     .await?
    ///     .flush()
    ///     .await?;
    pub async fn write(&mut self, data: &[u8]) -> Result<&mut Self> {
        self.buf.extend_from_slice(data);
        if self.buf.len() >= MAX_WRITE_BUF_SIZE {
            self.writer.ready_to_write().await?;
            self.flush_once()?;
        }

        Ok(self)
    }

    /// 将缓冲区的数据全部写入到内核
    /// 遇到Block的时候等待写事件再次就绪（内核缓冲区腾出空间时）
    pub async fn flush(&mut self) -> Result<()> {
        while !self.buf.is_empty() {
            self.writer.ready_to_write().await?;
            match self.flush_once() {
                Ok(()) => continue,
                Err(e) if e.is_blocked() => continue,
                Err(e) => {
                    log::error!("error in flush: {:?}", e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    fn flush_once(&mut self) -> Result<()> {
        if !self.buf.is_empty() {
            let size = self.writer.write(&self.buf)?;
            let _ = take_vec_at(&mut self.buf, size);
        }
        Ok(())
    }
}

impl<'a, W: TAsyncWrite> Drop for _AsyncBufWriterGuard<'a, W> {
    fn drop(&mut self) {
        let _ = err_log!(self.flush_once(), ".flush_once() in drop failed");
    }
}
