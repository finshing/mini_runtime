use std::{borrow::Cow, rc::Rc};

use crate::{
    BoxedFuture,
    result::Result,
    sync::mutex::{AsyncMutex, AsyncMutexGuard},
};

pub trait TAsyncWriteExt {
    fn async_write(&mut self, data: Cow<'_, [u8]>) -> BoxedFuture<'_, usize>;

    fn flush(&mut self) -> Result<()>;
}

#[derive(Clone)]
pub struct WriteExt<W: TAsyncWriteExt> {
    writer: Rc<AsyncMutex<W>>,
}

impl<W: TAsyncWriteExt> WriteExt<W> {
    pub fn new(writer: Rc<AsyncMutex<W>>) -> Self {
        Self { writer }
    }

    pub async fn lock(&self) -> WriteExtGuard<'_, W> {
        WriteExtGuard::new(self.writer.lock().await)
    }
}

pub struct WriteExtGuard<'a, W: TAsyncWriteExt> {
    writer: AsyncMutexGuard<'a, W>,
}

impl<'a, W: TAsyncWriteExt> WriteExtGuard<'a, W> {
    pub fn new(writer: AsyncMutexGuard<'a, W>) -> Self {
        Self { writer }
    }

    pub async fn write(&mut self, data: Cow<'_, [u8]>) -> Result<usize> {
        self.writer.async_write(data).await
    }
}

impl<'a, W: TAsyncWriteExt> Drop for WriteExtGuard<'a, W> {
    fn drop(&mut self) {
        if let Err(e) = self.writer.flush() {
            log::warn!("flush failed: {:?}", e);
        }
    }
}
