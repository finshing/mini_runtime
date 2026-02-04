use std::rc::Rc;

use memchr::memmem;

use crate::{
    BoxedFuture, config::READ_BUF_SIZE, result::Result, sync::mutex::AsyncMutex, variable_log,
};

pub trait TAsyncRead {
    // 判断读事件是否就绪
    fn ready_to_read(&mut self) -> BoxedFuture<'_, ()>;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
}

pub trait TAsyncBufRead: TAsyncRead {
    // 读取直到read_while返回Some时
    // read_while的参数是缓冲区引用
    fn read_util<'a>(
        &'a mut self,
        read_while: impl Fn(&[u8]) -> Option<usize> + 'a,
    ) -> BoxedFuture<'a, Vec<u8>>;

    // 读取一次直到block或eof
    fn read_once<'a>(&'a mut self, data: &'a mut Vec<u8>) -> BoxedFuture<'a, ()> {
        Box::pin(async {
            let mut buf = [0u8; READ_BUF_SIZE];

            self.ready_to_read().await?;
            loop {
                let size = self.read(&mut buf)?;
                if size == 0 {
                    return Ok(());
                }
                data.extend_from_slice(&buf[..size]);
            }
        })
    }
}

pub struct AsyncReader<R: TAsyncBufRead> {
    buf_reader: Rc<AsyncMutex<R>>,
}

impl<R: TAsyncBufRead> AsyncReader<R> {
    fn new(buf_reader: Rc<AsyncMutex<R>>) -> Self {
        Self { buf_reader }
    }

    pub async fn read_until(&mut self, end_at: &str) -> Result<Vec<u8>> {
        let end_at = end_at.as_bytes();

        let mut reader = self.buf_reader.lock().await;
        loop {
            // find找的的是匹配字符串首字符的offset
            match reader
                .read_util(|buf| memmem::find(buf, end_at).map(|size| size + end_at.len()))
                .await
            {
                Ok(data) => return Ok(data),
                Err(e) if e.is_blocked() => continue,
                Err(e) => return Err(e),
            }
        }
    }

    pub async fn read_until_exclusive(&mut self, end_at: &str) -> Result<Vec<u8>> {
        let mut data = self.read_until(end_at).await?;
        let _ = data.split_off(data.len() - end_at.len());
        Ok(data)
    }

    pub async fn read_exactly(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut reader = self.buf_reader.lock().await;
        loop {
            match reader
                .read_util(|buf| if buf.len() >= size { Some(size) } else { None })
                .await
            {
                Ok(data) => return Ok(data),
                Err(e) if e.is_blocked() => continue,
                Err(e) => return Err(e),
            }
        }
    }

    // 读取到eof
    pub async fn readall(&mut self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut reader = self.buf_reader.lock().await;
        loop {
            match variable_log!(debug @ reader.read_once(&mut data).await, ".readall()") {
                Err(e) if e.is_eof() => return Ok(data),
                Err(e) if !e.is_blocked() => return Err(e),
                _ => continue,
            }
        }
    }

    // 读取到eof或block时
    pub async fn read_once(&mut self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        let mut reader = self.buf_reader.lock().await;
        loop {
            match variable_log!(debug @ reader.read_once(&mut data).await, ".read_once()") {
                Err(e) if e.is_eof() | e.is_blocked() => return Ok(data),
                Err(e) => return Err(e),
                _ => continue,
            }
        }
    }
}

impl<R: TAsyncBufRead> From<Rc<AsyncMutex<R>>> for AsyncReader<R> {
    fn from(reader: Rc<AsyncMutex<R>>) -> Self {
        Self::new(reader)
    }
}
