use std::{
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Once,
    vec,
};

use mio::net::UdpSocket;

mod buf_reader;

use crate::{
    err_log,
    io_event::IoEvent,
    io_ext::{read::TAsyncRead, write::TAsyncWrite},
    result::{ErrorType, Result},
    runtime::register,
    udp::buf_reader::UdpBufReader,
};

pub struct Udp {
    io_event: Box<IoEvent>,
    socket: UdpSocket,
    server_addr: SocketAddr,
    buffer: UdpBufReader,
    once: Once,
}

impl Udp {
    pub fn new(server_addr: SocketAddr) -> Result<Self> {
        // 绑定本地随机端口（0 表示自动分配）
        let mut socket = UdpSocket::bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0))?;
        let io_event = IoEvent::new();
        register(vec![crate::io_event::Event::Write], &io_event, &mut socket)?;
        Ok(Self {
            io_event,
            socket,
            server_addr,
            buffer: UdpBufReader::default(),
            once: Once::new(),
        })
    }

    fn recv_once(&mut self) -> Result<()> {
        let mut result: Result<()> = Ok(());
        self.once.call_once(|| {
            result = self.buffer.init_from_udp_socket(&mut self.socket);
        });
        err_log!(result, "udp recv failed")
    }
}

impl TAsyncRead for Udp {
    fn ready_to_read(&mut self) -> crate::BoxedFuture<'_, ()> {
        Box::pin(async {
            self.io_event
                .reregister(&mut self.socket, crate::io_event::Event::Read)?
                .await;
            Ok(())
        })
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        // Udp不像Tcp可以重复读取同一批数据，因此需要设置足够大的缓冲区一次性读取
        self.recv_once()?;
        let size = self.buffer.read(buf)?;
        if size == 0 {
            return Err(ErrorType::Eof.into());
        }
        Ok(size)
    }
}

impl TAsyncWrite for Udp {
    fn ready_to_write(&mut self) -> crate::BoxedFuture<'_, ()> {
        Box::pin(async {
            self.io_event
                .reregister(&mut self.socket, crate::io_event::Event::Write)?
                .await;
            Ok(())
        })
    }

    fn write(&mut self, data: &[u8]) -> Result<usize> {
        Ok(self.socket.send_to(data, self.server_addr)?)
    }
}
