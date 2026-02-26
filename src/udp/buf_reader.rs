use std::ptr;

use mio::net::UdpSocket;

use crate::result::Result;

const UDP_READ_BUF: usize = 1024;

unsafe fn slice_copy(dst: &mut [u8], src: &[u8]) -> usize {
    let copy_size = dst.len().min(src.len());
    let dst_ptr = dst.as_ptr().cast_mut();
    let src_ptr = src.as_ptr();

    unsafe {
        ptr::copy_nonoverlapping(src_ptr, dst_ptr, copy_size);
    }

    copy_size
}

/// 将udp的一批数据全部读取到缓冲中，然后实现自己的Read trait
pub struct UdpBufReader {
    buf: [u8; UDP_READ_BUF],
    offset: usize,
    size: usize,
}

impl UdpBufReader {
    #[allow(unused)]
    pub fn init(&mut self, data: &[u8]) {
        self.size = unsafe { slice_copy(&mut self.buf, data) };
        self.offset = 0;
    }

    pub fn init_from_udp_socket(&mut self, socket: &mut UdpSocket) -> Result<()> {
        self.size = socket.recv(&mut self.buf)?;
        self.offset = 0;
        Ok(())
    }
}

impl std::io::Read for UdpBufReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read_size = unsafe { slice_copy(buf, &self.buf[self.offset..self.size]) };
        self.offset += read_size;

        Ok(read_size)
    }
}

impl Default for UdpBufReader {
    fn default() -> Self {
        Self {
            buf: [0u8; UDP_READ_BUF],
            offset: 0,
            size: 0,
        }
    }
}

#[cfg(test)]
mod test {
    use std::io::Read;

    use crate::{result::Result, udp::buf_reader::UdpBufReader};

    #[test]
    fn test_buf_reader() -> Result<()> {
        let mut reader = UdpBufReader::default();
        for i in 0..100 {
            let data = (0..i).collect::<Vec<u8>>();
            reader.init(&data);

            let mut bucket = Vec::new();
            let mut buf = [0u8; 4];
            loop {
                let n = reader.read(&mut buf)?;
                if n == 0 {
                    break;
                }
                bucket.extend_from_slice(&buf[..n]);
            }
            assert_eq!(data, bucket);
        }
        Ok(())
    }
}
