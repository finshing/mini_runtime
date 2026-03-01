use crate::{
    dns::dns_parse,
    io_ext::{read::AsyncReader, write::AsyncBufWriter},
    result::Result,
    timeout::ConnTimeout,
    web::conn::{SharedTcpConn, TcpConn, new_tcp_conn},
};

pub struct ClientBuilder {
    host: String,
    port: u16,
    timeout: ConnTimeout,
}

impl ClientBuilder {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_owned(),
            port,
            timeout: ConnTimeout::new(None),
        }
    }

    pub fn update_timeout(mut self, mut updater: impl FnMut(&mut ConnTimeout)) -> Self {
        updater(&mut self.timeout);
        self
    }

    pub async fn connect(self) -> Result<Client> {
        let addr = dns_parse(&self.host, self.port).await?;
        let tcp_stream = mio::net::TcpStream::connect(addr)?;
        Ok(Client {
            conn: new_tcp_conn(tcp_stream, self.timeout)?,
        })
    }
}

pub struct Client {
    conn: SharedTcpConn,
}

impl Client {
    pub fn writer(&self) -> AsyncBufWriter<TcpConn> {
        self.conn.clone().into()
    }

    pub fn reader(&self) -> AsyncReader<TcpConn> {
        self.conn.clone().into()
    }
}
