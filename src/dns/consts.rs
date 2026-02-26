use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

// DNS 事务 ID 长度（2 字节）
// const DNS_HEADER_ID_LEN: usize = 2;
// DNS 头部总长度（12 字节）
pub const DNS_HEADER_TOTAL_LEN: usize = 12;
// DNS 默认 UDP 端口
pub const DNS_DEFAULT_PORT: u16 = 53;
// DNS 查询超时（5 秒）
pub const DNS_TIMEOUT: Duration = Duration::from_millis(5000);

pub const DNS_SERVER: SocketAddr = SocketAddr::V4(SocketAddrV4::new(
    // Ipv4Addr::new(8, 8, 4, 4),
    Ipv4Addr::new(119, 29, 29, 29),
    DNS_DEFAULT_PORT,
));

pub const DNS_CACHE_FILE: &str = ".dns_cache";

pub const DNS_CACHE_TTL: Duration = Duration::from_secs(10);
