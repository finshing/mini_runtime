use std::{
    borrow::Cow,
    fmt::Display,
    io::{Read, Write},
    os::fd::AsRawFd,
};

use mio::net::TcpStream;

use crate::{
    io_event::IoEvent,
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

    pub async fn ready_to_read(&mut self) -> Result<()> {
        self.io_event
            .reregister(&mut self.tcp_stream, crate::io_event::Event::Read)?
            .await;
        Ok(())
    }

    pub async fn ready_to_write(&mut self) -> Result<()> {
        self.io_event
            .reregister(&mut self.tcp_stream, crate::io_event::Event::Write)?
            .await;
        Ok(())
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let size = self.tcp_stream.read(buf)?;
        if size == 0 {
            return Err(ErrorType::Eof.into());
        }
        Ok(size)
    }

    pub fn write(&mut self, data: Cow<'_, [u8]>) -> Result<usize> {
        Ok(self.tcp_stream.write(&data)?)
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
