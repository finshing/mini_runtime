use mini_runtime::{
    config::{ECHO_SERVER_IP, ECHO_SERVER_PORT},
    io_ext::{read::TAsyncRead, write::TAsyncWrite},
    result::{ErrorType, Result},
    tcp::{listener::Listener, stream::Stream},
};

#[rt_entry::main(log_level = "debug")]
async fn main() -> Result<()> {
    let mut listener = Listener::new(ECHO_SERVER_IP, ECHO_SERVER_PORT)?;

    loop {
        listener.ready().await?;
        loop {
            match listener.accept() {
                Ok((tcp_stream, addr)) => {
                    let stream = Stream::new(tcp_stream)?;
                    log::info!("success build connection {} from {}", stream, addr);
                    spawn!(echo(stream));
                }
                Err(e) if matches!(e.err_type(), ErrorType::Blocked) => {
                    log::trace!("listener accept  blocked, break here");
                    break;
                }
                Err(e) => {
                    log::warn!("fail to accept connection: {:?}", e);
                }
            }
        }
    }
}

async fn echo(mut stream: Stream) -> Result<()> {
    stream.ready_to_read().await?;
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf)?;

    stream.ready_to_write().await?;
    stream.write(&buf[..n])?;
    Ok(())
}
