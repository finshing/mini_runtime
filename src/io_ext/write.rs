use std::rc::Rc;

use crate::{
    BoxedFuture, err_log,
    result::Result,
    sync::mutex::{AsyncMutex, AsyncMutexGuard},
};

pub trait TAsyncWrite {
    fn async_write<'a>(&'a mut self, data: &'a [u8]) -> BoxedFuture<'a, usize>;

    fn flush(&mut self) -> Result<()>;
}

#[derive(Clone)]
pub struct AsyncWriter<W: TAsyncWrite> {
    writer: Rc<AsyncMutex<W>>,
}

impl<W: TAsyncWrite> AsyncWriter<W> {
    fn new(writer: Rc<AsyncMutex<W>>) -> Self {
        Self { writer }
    }

    pub async fn lock(&self) -> AsyncWriterGuard<'_, W> {
        AsyncWriterGuard::new(self.writer.lock().await)
    }
}

impl<W: TAsyncWrite> From<Rc<AsyncMutex<W>>> for AsyncWriter<W> {
    fn from(writer: Rc<AsyncMutex<W>>) -> Self {
        Self::new(writer)
    }
}

pub struct AsyncWriterGuard<'a, W: TAsyncWrite> {
    writer: AsyncMutexGuard<'a, W>,
}

impl<'a, W: TAsyncWrite> AsyncWriterGuard<'a, W> {
    pub fn new(writer: AsyncMutexGuard<'a, W>) -> Self {
        Self { writer }
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<()> {
        let mut offset = 0usize;
        loop {
            match self.writer.async_write(data[offset..].into()).await {
                Ok(n) => {
                    offset += n;
                    if offset == data.len() {
                        return Ok(());
                    }
                }
                Err(e) if e.is_blocked() => continue, // 写缓冲区满了
                Err(e) => return Err(e),
            }
        }
    }
}

impl<'a, W: TAsyncWrite> Drop for AsyncWriterGuard<'a, W> {
    fn drop(&mut self) {
        let _ = err_log!(self.writer.flush(), "flush failed");
    }
}
