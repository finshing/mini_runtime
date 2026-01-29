use crate::{
    io_ext::{read::AsyncReader, write::AsyncWriter},
    result::Result,
    web::conn::{_Conn, Conn, new_conn},
};

pub struct Client {
    conn: Conn,
}

impl Client {
    pub fn connect(ip: &str, port: usize) -> Result<Self> {
        let tcp_stream = mio::net::TcpStream::connect(format!("{}:{}", ip, port).parse()?)?;
        Ok(Self {
            conn: new_conn(tcp_stream)?,
        })
    }

    pub fn writer(&self) -> AsyncWriter<_Conn> {
        self.conn.clone().into()
    }

    pub fn reader(&self) -> AsyncReader<_Conn> {
        self.conn.clone().into()
    }
}
