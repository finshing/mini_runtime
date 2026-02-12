use core::time;

use crate::{
    BoxedFutureWithError, err_log,
    result::Result,
    runtime::spawn,
    select,
    signal::{StopWaker, set_max_wait_duration},
    sleep,
    tcp::listener::Listener,
    timeout::ConnTimeout,
    web::conn::{Conn, new_conn},
};

pub struct Server<E, H>
where
    H: Fn(Conn) -> BoxedFutureWithError<'static, (), E>,
{
    listener: Listener,
    conn_handler: H,
    timeout: ConnTimeout,
    stop_waker: StopWaker,
}

impl<E, H> Server<E, H>
where
    H: Fn(Conn) -> BoxedFutureWithError<'static, (), E>,
{
    pub fn new(ip: &str, port: usize, conn_handler: H) -> Result<Self> {
        Ok(Self {
            listener: Listener::new(ip, port)?,
            conn_handler,
            timeout: ConnTimeout::new(None),
            stop_waker: StopWaker::default(),
        })
    }

    pub fn update_timeout(&mut self, mut updater: impl FnMut(&mut ConnTimeout)) -> &mut Self {
        updater(&mut self.timeout);
        self
    }

    pub fn set_max_wait_time(&mut self, wait_time: time::Duration) -> &mut Self {
        set_max_wait_duration(wait_time);
        self
    }

    pub async fn run(&mut self) -> Result<()> {
        let dur = time::Duration::from_secs(2);
        loop {
            select! {
                Err(e) = self.listener.ready() => {
                    log::warn!("server .accept() error: {:?}", e);
                    continue;
                } || {
                    log::debug!("server .accept() ready");
                },
                _ = sleep(dur) => {
                    log::debug!("server heartbeat");
                },
                _ = self.stop_waker.wait() => {
                    log::info!("server stopped");
                    return Ok(())
                }
            }
            self.build_connections();
        }
    }

    fn build_connections(&mut self) {
        loop {
            match self.listener.accept() {
                Ok((tcp_stream, addr)) => {
                    log::debug!("build connection with {}", addr);
                    if let Ok(conn) = err_log!(
                        new_conn(tcp_stream, self.timeout.clone()),
                        "Conn::new failed"
                    ) {
                        spawn((self.conn_handler)(conn));
                    }
                }
                Err(e) if e.is_blocked() => return,
                _ => continue,
            }
        }
    }
}
