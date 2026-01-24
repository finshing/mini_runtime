use std::{fmt::Display, os::fd::AsRawFd};

use crate::{
    io_event::{Event, IoEvent},
    result::Result,
    runtime::{deregister, register},
};

pub struct Listener {
    tcp_listener: mio::net::TcpListener,
    io_event: Box<IoEvent>,
}

impl Listener {
    pub fn new(ip: &str, port: usize) -> Result<Self> {
        let host = format!("{}:{}", ip, port);
        let mut tcp_listener = mio::net::TcpListener::bind(host.parse()?)?;
        let io_event = IoEvent::new();

        register(vec![Event::Read], &io_event, &mut tcp_listener)?;
        log::info!("listen at {}", host);

        Ok(Self {
            tcp_listener,
            io_event,
        })
    }

    pub async fn ready(&mut self) -> Result<()> {
        self.io_event
            .reregister(&mut self.tcp_listener, Event::Read)?
            .await;
        Ok(())
    }

    pub fn accept(&mut self) -> Result<(mio::net::TcpStream, std::net::SocketAddr)> {
        Ok(self.tcp_listener.accept()?)
    }
}

impl Display for Listener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "listener-{}", self.tcp_listener.as_raw_fd())
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        if let Err(e) = deregister(&mut self.tcp_listener) {
            log::warn!("deregister {} failed: {:?}", self, e);
        }

        log::debug!("{} closed", self);
    }
}
