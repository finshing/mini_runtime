use std::{net::SocketAddr, rc::Rc};

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
    udp::Udp,
};

pub type SharedTcpConn = Rc<AsyncMutex<TcpConn>>;

pub type TcpConn = _Conn<Stream>;

pub type SharedUdpConn = Rc<AsyncMutex<UdpConn>>;

pub type UdpConn = _Conn<Udp>;

pub fn new_tcp_conn(
    tcp_stream: mio::net::TcpStream,
    timeout: ConnTimeout,
) -> Result<SharedTcpConn> {
    Ok(Rc::new(AsyncMutex::new(_Conn::<Stream>::tcp_conn(
        tcp_stream, timeout,
    )?)))
}

pub fn new_udp_conn(server_addr: SocketAddr, timeout: ConnTimeout) -> Result<SharedUdpConn> {
    Ok(Rc::new(AsyncMutex::new(_Conn::<Udp>::udp_conn(
        server_addr,
        timeout,
    )?)))
}

pub struct _Conn<T: TAsyncRead + TAsyncWrite> {
    inner: T,
    buf: Vec<u8>,
    timeout: ConnTimeout,
}

impl<T: TAsyncRead + TAsyncWrite> _Conn<T> {
    fn tcp_conn(tcp_stream: mio::net::TcpStream, timeout: ConnTimeout) -> Result<TcpConn> {
        Ok(_Conn {
            inner: Stream::new(tcp_stream)?,
            buf: Vec::new(),
            timeout,
        })
    }

    fn udp_conn(server_addr: SocketAddr, timeout: ConnTimeout) -> Result<UdpConn> {
        Ok(_Conn {
            inner: Udp::new(server_addr)?,
            buf: Vec::new(),
            timeout,
        })
    }

    pub fn set_timeout(&mut self, timeout: ConnTimeout) {
        self.timeout = timeout;
    }

    fn take(&mut self, at: usize) -> Vec<u8> {
        assert!(self.buf.len() >= at);

        take_vec_at(&mut self.buf, at)
    }
}

impl<T: TAsyncRead + TAsyncWrite> TAsyncRead for _Conn<T> {
    fn ready_to_read(&mut self) -> crate::BoxedFuture<'_, ()> {
        Box::pin(async {
            select! {
                _ = self.timeout.timeout() => {
                    return Err(ErrorType::Timeout.into());
                },
                _ = self.timeout.read_timeout() => {
                    return Err(ErrorType::ReadTimeout.into());
                },
                result = self.inner.ready_to_read() => {
                    err_log!(result, ".ready_to_read() failed")?;
                }
            }

            Ok(())
        })
    }

    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(buf)
    }
}

impl<T: TAsyncRead + TAsyncWrite> TAsyncBufRead for _Conn<T> {
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

impl<T: TAsyncRead + TAsyncWrite> TAsyncWrite for _Conn<T> {
    fn ready_to_write(&mut self) -> crate::BoxedFuture<'_, ()> {
        Box::pin(async {
            select! {
                _ = self.timeout.timeout() =>  {
                    return Err(ErrorType::Timeout.into());
                },
                _ = self.timeout.write_timeout() => {
                    return Err(ErrorType::WriteTimeout.into());
                },
                result = self.inner.ready_to_write() => {
                    err_log!(result, ".ready_to_write() failed")?;
                }
            }

            Ok(())
        })
    }

    fn write(&mut self, data: &[u8]) -> Result<usize> {
        self.inner.write(data)
    }
}
