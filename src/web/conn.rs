use std::rc::Rc;

use crate::{
    config::READ_BUF_SIZE,
    io_ext::{
        read::{TAsyncBufRead, TAsyncRead},
        write::TAsyncWrite,
    },
    result::Result,
    sync::mutex::AsyncMutex,
    tcp::stream::Stream,
};

pub type Conn = Rc<AsyncMutex<_Conn>>;

pub fn new_conn(tcp_stream: mio::net::TcpStream) -> Result<Conn> {
    Ok(Rc::new(AsyncMutex::new(_Conn::new(tcp_stream)?)))
}

pub struct _Conn {
    stream: Stream,

    buf: Vec<u8>,
}

impl _Conn {
    pub fn new(tcp_stream: mio::net::TcpStream) -> Result<Self> {
        Ok(Self {
            stream: Stream::new(tcp_stream)?,
            buf: Vec::new(),
        })
    }

    fn take(&mut self, at: usize) -> Vec<u8> {
        assert!(self.buf.len() >= at);

        let left = self.buf.split_off(at);
        std::mem::replace(&mut self.buf, left)
    }
}

impl TAsyncRead for _Conn {
    fn ready_to_read(&mut self) -> crate::BoxedFuture<'_, ()> {
        self.stream.ready_to_read()
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
        self.stream.ready_to_write()
    }

    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.stream.write(data)
    }
}
