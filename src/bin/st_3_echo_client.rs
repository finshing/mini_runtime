use mini_runtime::{
    config::{ECHO_SERVER_IP, ECHO_SERVER_PORT},
    io_ext::{read::TAsyncRead, write::TAsyncWrite},
    result::Result,
    tcp::stream::Stream,
};
use mio::net::TcpStream;

#[rt_entry::main]
async fn main() -> Result<()> {
    let host = format!("{}:{}", ECHO_SERVER_IP, ECHO_SERVER_PORT);
    let tcp_stream = TcpStream::connect(host.parse()?)?;
    let mut stream = Stream::new(tcp_stream)?;

    stream.async_write("hello, mini runtime".as_bytes()).await?;

    // 这里给够缓存大小，并认为可以一次性读取到全部响应内容
    let mut buf = [0u8; 32];
    TAsyncRead::ready(&mut stream).await?;
    let size = stream.async_read(&mut buf)?;
    let resp = String::from_utf8_lossy(&buf[..size]);
    log::info!("get response: {:?}", resp);
    Ok(())
}
