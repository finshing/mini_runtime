use std::rc::Rc;

use crate::{
    config::READ_BUF_SIZE,
    err_log,
    helper::take_vec_at,
    io_ext::{
        read::{TAsyncBufRead, TAsyncRead},
        write::TAsyncWrite,
    },
    result::{ErrorType, Result},
    select,
    sync::mutex::AsyncMutex,
    tcp::stream::Stream,
    timeout::ConnTimeout,
};

pub type Conn = Rc<AsyncMutex<_Conn>>;

pub fn new_conn(tcp_stream: mio::net::TcpStream, timeout: ConnTimeout) -> Result<Conn> {
    Ok(Rc::new(AsyncMutex::new(_Conn::new(tcp_stream, timeout)?)))
}

pub struct _Conn {
    stream: Stream,
    buf: Vec<u8>,
    timeout: ConnTimeout,
}

impl _Conn {
    pub fn new(tcp_stream: mio::net::TcpStream, timeout: ConnTimeout) -> Result<Self> {
        Ok(Self {
            stream: Stream::new(tcp_stream)?,
            buf: Vec::new(),
            timeout,
        })
    }

    fn take(&mut self, at: usize) -> Vec<u8> {
        assert!(self.buf.len() >= at);

        take_vec_at(&mut self.buf, at)
    }
}

impl TAsyncRead for _Conn {
    fn ready_to_read(&mut self) -> crate::BoxedFuture<'_, ()> {
        Box::pin(async {
            select! {
                _ = self.timeout.timeout() => {
                    return Err(ErrorType::Timeout.into());
                },
                _ = self.timeout.read_timeout() => {
                    return Err(ErrorType::ReadTimeout.into());
                },
                result = self.stream.ready_to_read() => {
                    err_log!(result, ".ready_to_read() failed")?;
                }
            }

            Ok(())
        })
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.stream.read(buf)
    }
}

impl TAsyncBufRead for _Conn {
    fn read_util<'a>(
        &'a mut self,
        read_while: impl Fn(&[u8]) -> Option<usize> + 'a,
    ) -> crate::BoxedFuture<'a, Vec<u8>> {
        Box::pin(async move {
            if let Some(at) = read_while(&self.buf) {
                return Ok(self.take(at));
            }

            let mut buf = [0u8; READ_BUF_SIZE];
            self.ready_to_read().await?;
            loop {
                let size = self.read(&mut buf)?;
                self.buf.extend_from_slice(&buf[..size]);
                if let Some(at) = read_while(&self.buf) {
                    return Ok(self.take(at));
                }
            }
        })
    }
}

impl TAsyncWrite for _Conn {
    fn ready_to_write(&mut self) -> crate::BoxedFuture<'_, ()> {
        Box::pin(async {
            select! {
                _ = self.timeout.timeout() =>  {
                    return Err(ErrorType::Timeout.into());
                },
                _ = self.timeout.write_timeout() => {
                    return Err(ErrorType::WriteTimeout.into());
                },
                result = self.stream.ready_to_write() => {
                    err_log!(result, ".ready_to_write() failed")?;
                }
            }

            Ok(())
        })
    }

    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.stream.write(data)
    }
}
