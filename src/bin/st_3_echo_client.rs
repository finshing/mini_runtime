use mini_runtime::{
    config::{ECHO_SERVER_IP, ECHO_SERVER_PORT},
    result::Result,
    tcp::stream::Stream,
};
use mio::net::TcpStream;

#[rt_entry::main]
async fn main() -> Result<()> {
    let host = format!("{}:{}", ECHO_SERVER_IP, ECHO_SERVER_PORT);
    let tcp_stream = TcpStream::connect(host.parse()?)?;
    let mut stream = Stream::new(tcp_stream)?;

    stream.ready_to_write().await?;
    stream
        .write("hello, mini runtime".as_bytes().into())
        .await?;

    // 这里给够缓存大小，并认为可以一次性读取到全部响应内容
    let mut buf = [0u8; 32];
    stream.ready_to_read().await?;
    let size = stream.read(&mut buf).await?;
    let resp = String::from_utf8_lossy(&buf[..size]);
    log::info!("get response: {:?}", resp);
    Ok(())
}
