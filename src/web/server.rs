use crate::{
    BoxedFutureWithError, err_log,
    result::Result,
    runtime::spawn,
    tcp::listener::Listener,
    web::conn::{Conn, new_conn},
};

pub struct Server<E, H>
where
    H: Fn(Conn) -> BoxedFutureWithError<'static, (), E>,
{
    listener: Listener,

    conn_handler: H,
}

impl<E, H> Server<E, H>
where
    H: Fn(Conn) -> BoxedFutureWithError<'static, (), E>,
{
    pub fn new(ip: &str, port: usize, conn_handler: H) -> Result<Self> {
        Ok(Self {
            listener: Listener::new(ip, port)?,
            conn_handler,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        loop {
            self.listener.ready().await?;
            self.build_connections();
        }
    }

    fn build_connections(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((tcp_stream, addr)) => {
                    log::debug!("build connection with {}", addr);
                    if let Ok(conn) = err_log!(new_conn(tcp_stream), "Conn::new failed") {
                        spawn((self.conn_handler)(conn));
                    }
                }
                Err(e) if e.is_blocked() => return,
                _ => continue,
            }
        }
    }
}
