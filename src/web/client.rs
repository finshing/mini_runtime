use crate::{
    io_ext::{read::AsyncReader, write::AsyncBufWriter},
    result::Result,
    timeout::ConnTimeout,
    web::conn::{_Conn, Conn, new_conn},
};

pub struct ClientBuilder {
    host: String,
    timeout: ConnTimeout,
}

impl ClientBuilder {
    pub fn new(ip: &str, port: usize) -> Self {
        Self {
            host: format!("{}:{}", ip, port),
            timeout: ConnTimeout::new(None),
        }
    }

    pub fn update_timeout(mut self, mut updater: impl FnMut(&mut ConnTimeout)) -> Self {
        updater(&mut self.timeout);
        self
    }

    pub fn connect(self) -> Result<Client> {
        let tcp_stream = mio::net::TcpStream::connect(self.host.parse()?)?;
        Ok(Client {
            conn: new_conn(tcp_stream, self.timeout)?,
        })
    }
}

pub struct Client {
    conn: Conn,
}

impl Client {
    pub fn writer(&self) -> AsyncBufWriter<_Conn> {
        self.conn.clone().into()
    }

    pub fn reader(&self) -> AsyncReader<_Conn> {
        self.conn.clone().into()
    }
}
