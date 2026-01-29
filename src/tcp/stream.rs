use std::{
    fmt::Display,
    io::{Read, Write},
    os::fd::{AsRawFd, RawFd},
};

use mio::net::TcpStream;

use crate::{
    io_event::IoEvent,
    io_ext::{read::TAsyncRead, write::TAsyncWrite},
    result::{ErrorType, Result},
    runtime::{deregister, register},
};

pub struct Stream {
    tcp_stream: TcpStream,
    io_event: Box<IoEvent>,
}

impl Stream {
    pub fn new(mut tcp_stream: TcpStream) -> Result<Self> {
        let io_event = IoEvent::new();
        register(
            vec![crate::io_event::Event::Read, crate::io_event::Event::Write],
            &io_event,
            &mut tcp_stream,
        )?;
        Ok(Self {
            tcp_stream,
            io_event,
        })
    }

    pub fn fd(&self) -> RawFd {
        self.tcp_stream.as_raw_fd()
    }
}

impl TAsyncRead for Stream {
    fn ready(&mut self) -> crate::BoxedFuture<'_, ()> {
        Box::pin(async {
            self.io_event
                .reregister(&mut self.tcp_stream, crate::io_event::Event::Read)?
                .await;
            Ok(())
        })
    }

    fn async_read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let size = self.tcp_stream.read(buf)?;
        if size == 0 {
            return Err(ErrorType::Eof.into());
        }
        Ok(size)
    }
}

impl TAsyncWrite for Stream {
    fn async_write<'a>(&'a mut self, data: &'a [u8]) -> crate::BoxedFuture<'a, usize> {
        Box::pin(async {
            self.io_event
                .reregister(&mut self.tcp_stream, crate::io_event::Event::Write)?
                .await;

            Ok(self.tcp_stream.write(data)?)
        })
    }

    fn flush(&mut self) -> Result<()> {
        Ok(self.tcp_stream.flush()?)
    }
}

impl Display for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stream-{}", self.tcp_stream.as_raw_fd())
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        if let Err(e) = deregister(&mut self.tcp_stream) {
            log::warn!("deregister {} failed: {:?}", self, e);
        }
        log::debug!("{} disconnected", self);
    }
}
