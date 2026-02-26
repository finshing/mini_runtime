use std::{
    fmt::Display,
    io::{self, Cursor, Read},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use crate::{
    dns::consts::DNS_HEADER_TOTAL_LEN,
    result::{ErrorType, Result},
};

// 压缩标识，通过下一个字节获取压缩数据的偏移
const COMPRESS_FLAG: u8 = 192;

/// @param domain: 待查询的域名
/// @param id: 随机生成的id作为事务id
/// @param record_type: 查询类型
pub fn build_dns_query(domain: &str, id: u16, record_type: QRecordType) -> io::Result<Vec<u8>> {
    let mut buf = Vec::new();

    // 1. 写入 DNS 头部
    let header = DnsHeader::new_query(id);
    buf.extend_from_slice(&header.to_bytes()?);

    // 2. 写入查询名（QNAME）：点分域名转 "长度+标签" 格式，末尾加 0
    for label in domain.split('.') {
        let label_len = label.len() as u8;
        buf.push(label_len);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0); // QNAME 结束符

    // 3. 写入 QTYPE（A 记录 = 1）
    buf.write_u16::<BigEndian>(record_type.to_qtype())?;

    // 4. 写入 QCLASS（IN 类 = 1）
    buf.write_u16::<BigEndian>(1)?;

    Ok(buf)
}

#[allow(unused)]
#[derive(Debug)]
pub struct DnsResponse {
    header: DnsHeader,
    question: Question,
    pub records: Vec<EntireRecord>,
}

impl DnsResponse {
    pub fn deserialize(response: &[u8], expected_id: u16) -> Result<Self> {
        let mut cursor = Cursor::new(response);

        // 1. 解析头部
        let header = DnsHeader::from_bytes(&response[0..DNS_HEADER_TOTAL_LEN])?;
        if header.id != expected_id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DNS 事务 ID 不匹配",
            ))?;
        }
        if header.ancount == 0 {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "DNS 响应无回答记录",
            ))?;
        }
        cursor.set_position(DNS_HEADER_TOTAL_LEN as u64);

        let question = Question::deserialize(&mut cursor)?;
        let records = RecordReader::new(&mut cursor)
            .filter_map(|r| match r {
                Ok(r) => Some(r),
                Err(e) => {
                    log::warn!("parse record failed: {:?}", e);
                    None
                }
            })
            .collect::<Vec<_>>();

        Ok(Self {
            header,
            question,
            records,
        })
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QRecordType {
    A,    // IPv4 记录 (QTYPE=1)
    AAAA, // IPv6 记录 (QTYPE=28)
    CNAME,
    UNKNOWN,
}

impl QRecordType {
    fn to_qtype(self) -> u16 {
        match self {
            QRecordType::A => 1,
            QRecordType::AAAA => 28,
            _ => 0,
        }
    }
}

impl From<u16> for QRecordType {
    fn from(value: u16) -> Self {
        match value {
            1 => Self::A,
            5 => Self::CNAME,
            28 => Self::AAAA,
            _ => Self::UNKNOWN,
        }
    }
}

impl Display for QRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::A => write!(f, "A"),
            Self::AAAA => write!(f, "AAAA"),
            Self::CNAME => write!(f, "CNAME"),
            Self::UNKNOWN => write!(f, "UNKNOWN"),
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QClassType {
    IN,
    UNKNOWN,
}

impl From<u16> for QClassType {
    fn from(value: u16) -> Self {
        match value {
            1 => Self::IN,
            _ => Self::UNKNOWN,
        }
    }
}

/// DNS 头部（RFC 1035）
#[derive(Debug)]
pub struct DnsHeader {
    id: u16,      // 事务 ID
    flags: u16,   // 标志位（查询/响应、递归请求等）
    qdcount: u16, // 查询记录数
    ancount: u16, // 回答记录数
    nscount: u16, // 授权记录数
    arcount: u16, // 附加记录数
}

impl DnsHeader {
    /// 构造 DNS 查询头部（A 记录，递归请求）
    pub fn new_query(id: u16) -> Self {
        Self {
            id,
            flags: 0x0100, // 0x0100 = 递归请求 (RD=1)，查询 (QR=0)
            qdcount: 1,    // 1 个查询记录
            ancount: 0,
            nscount: 0,
            arcount: 0,
        }
    }

    /// 序列化为字节（大端序）
    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(DNS_HEADER_TOTAL_LEN);
        buf.write_u16::<BigEndian>(self.id)?;
        buf.write_u16::<BigEndian>(self.flags)?;
        buf.write_u16::<BigEndian>(self.qdcount)?;
        buf.write_u16::<BigEndian>(self.ancount)?;
        buf.write_u16::<BigEndian>(self.nscount)?;
        buf.write_u16::<BigEndian>(self.arcount)?;
        Ok(buf)
    }

    /// 从字节反序列化（大端序）
    pub fn from_bytes(mut buf: &[u8]) -> io::Result<Self> {
        Ok(Self {
            id: buf.read_u16::<BigEndian>()?,
            flags: buf.read_u16::<BigEndian>()?,
            qdcount: buf.read_u16::<BigEndian>()?,
            ancount: buf.read_u16::<BigEndian>()?,
            nscount: buf.read_u16::<BigEndian>()?,
            arcount: buf.read_u16::<BigEndian>()?,
        })
    }
}

fn read_name(cursor: &mut Cursor<&[u8]>) -> Result<String> {
    let name_vec = read_name_vec(cursor)?;
    Ok(String::from_utf8(name_vec)?)
}

fn read_name_vec(cursor: &mut Cursor<&[u8]>) -> Result<Vec<u8>> {
    let mut buf = [0u8; 256];
    let mut name = Vec::new();
    loop {
        match cursor.read_u8()? {
            // 压缩
            COMPRESS_FLAG => {
                let offset = cursor.read_u8()?;
                let mut cursor_clone = cursor.clone();
                cursor_clone.set_position(offset as u64);
                name.extend(read_name_vec(&mut cursor_clone)?);
                break;
            }
            0 => {
                break;
            }
            n => {
                if !name.is_empty() {
                    name.push(b'.');
                }
                cursor.read_exact(&mut buf[..n as usize])?;
                name.extend_from_slice(&buf[..n as usize]);
            }
        }
    }

    Ok(name)
}

#[allow(unused)]
#[derive(Debug)]
pub struct Question {
    name: String,
    record_type: QRecordType,
    class_type: QClassType,
}

impl Question {
    fn deserialize(cursor: &mut Cursor<&[u8]>) -> Result<Self> {
        let name = read_name(cursor)?;
        let record_type = cursor.read_u16::<BigEndian>()?;
        let class_type = cursor.read_u16::<BigEndian>()?;
        Ok(Self {
            name,
            record_type: record_type.into(),
            class_type: class_type.into(),
        })
    }
}

struct RecordReader<'a> {
    cursor: &'a mut Cursor<&'a [u8]>,
}

impl<'a> RecordReader<'a> {
    fn new(cursor: &'a mut Cursor<&'a [u8]>) -> Self {
        Self { cursor }
    }
}

impl<'a> Iterator for RecordReader<'a> {
    type Item = Result<EntireRecord>;

    fn next(&mut self) -> Option<Self::Item> {
        let record = Record::deserialize(self.cursor)
            .and_then(|record| Record::into_entire_record(record, self.cursor));
        match record {
            Ok(record) => Some(Ok(record)),
            Err(e) if e.is_eof() => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub struct Record {
    name: String,
    record_type: QRecordType,
    class_type: QClassType,
    ttl: u32,
    length: u16,
}

impl Record {
    pub fn deserialize(cursor: &mut Cursor<&[u8]>) -> Result<Self> {
        let name = read_name(cursor)?;
        let rt = QRecordType::from(cursor.read_u16::<BigEndian>()?);
        let ct = QClassType::from(cursor.read_u16::<BigEndian>()?);
        let ttl = cursor.read_u32::<BigEndian>()?;
        let length = cursor.read_u16::<BigEndian>()?;

        Ok(Self {
            name,
            record_type: rt,
            class_type: ct,
            ttl,
            length,
        })
    }

    fn into_entire_record(self, cursor: &mut Cursor<&[u8]>) -> Result<EntireRecord> {
        let length = self.length as usize;
        match self.record_type {
            QRecordType::A => {
                if self.length != 4 {
                    return Err(ErrorType::DnsParseFailed(format!(
                        "length not compat for {}",
                        self.record_type
                    ))
                    .into());
                }
                let mut buf = [0u8; 4];
                cursor.read_exact(&mut buf)?;
                Ok(EntireRecord::V4(self, Ipv4Addr::from(buf)))
            }
            QRecordType::AAAA => {
                if self.length != 16 {
                    return Err(ErrorType::DnsParseFailed(format!(
                        "length not compat for {}",
                        self.record_type
                    ))
                    .into());
                }
                let mut buf = [0u8; 16];
                cursor.read_exact(&mut buf)?;
                Ok(EntireRecord::V6(self, Ipv6Addr::from(buf)))
            }
            QRecordType::CNAME => Ok(EntireRecord::Cname(self, read_name(cursor)?)),
            QRecordType::UNKNOWN => {
                let mut buf = [0u8; 256];
                cursor.read_exact(&mut buf[..length])?;
                Ok(EntireRecord::Unkowned(self, buf[..length].to_vec()))
            }
        }
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum EntireRecord {
    V4(Record, Ipv4Addr),
    V6(Record, Ipv6Addr),
    Cname(Record, String),
    Unkowned(Record, Vec<u8>),
}

impl EntireRecord {
    pub fn get_ip_addr(&self) -> Option<IpAddr> {
        match self {
            Self::V4(_, v4) => Some(IpAddr::V4(*v4)),
            Self::V6(_, v6) => Some(IpAddr::V6(*v6)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, vec};

    use byteorder::{BigEndian, ReadBytesExt};

    use crate::dns::protocol::DnsResponse;

    #[test]
    fn test_cursor() {
        let a = [0u8; 3];
        let mut cursor = Cursor::new(&a);
        cursor.read_u32::<BigEndian>().unwrap();
    }

    #[test]
    fn test_str_decode() {
        let v = vec![
            119, 119, 119, 46, 98, 105, 108, 105, 98, 105, 108, 105, 46, 99, 111, 109, 46,
        ];
        let result = String::from_utf8(v);

        println!("result: {:?}", result);
    }

    #[test]
    fn test_v6() {
        let response = [
            198, 11, 129, 128, 0, 1, 0, 9, 0, 0, 0, 0, 3, 119, 119, 119, 8, 98, 105, 108, 105, 98,
            105, 108, 105, 3, 99, 111, 109, 0, 0, 28, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 1, 34, 0,
            15, 1, 97, 1, 119, 8, 98, 105, 108, 105, 99, 100, 110, 49, 192, 25, 192, 46, 0, 28, 0,
            1, 0, 0, 0, 90, 0, 16, 36, 8, 135, 47, 0, 32, 0, 11, 0, 0, 0, 0, 0, 0, 0, 19, 192, 46,
            0, 28, 0, 1, 0, 0, 0, 90, 0, 16, 36, 8, 135, 38, 17, 0, 1, 0, 0, 0, 0, 0, 0, 2, 0, 23,
            192, 46, 0, 28, 0, 1, 0, 0, 0, 90, 0, 16, 36, 8, 135, 47, 0, 32, 0, 11, 0, 0, 0, 0, 0,
            0, 0, 21, 192, 46, 0, 28, 0, 1, 0, 0, 0, 90, 0, 16, 36, 8, 135, 47, 0, 32, 0, 11, 0, 0,
            0, 0, 0, 0, 0, 20, 192, 46, 0, 28, 0, 1, 0, 0, 0, 90, 0, 16, 36, 8, 135, 38, 17, 0, 1,
            0, 0, 0, 0, 0, 0, 2, 0, 24, 192, 46, 0, 28, 0, 1, 0, 0, 0, 90, 0, 16, 36, 8, 135, 38,
            17, 0, 1, 0, 0, 0, 0, 0, 0, 2, 0, 33, 192, 46, 0, 28, 0, 1, 0, 0, 0, 90, 0, 16, 36, 8,
            135, 38, 17, 0, 1, 0, 0, 0, 0, 0, 0, 2, 0, 32, 192, 46, 0, 28, 0, 1, 0, 0, 0, 90, 0,
            16, 36, 8, 135, 38, 17, 0, 1, 0, 0, 0, 0, 0, 0, 2, 0, 18,
        ];

        let resp = DnsResponse::deserialize(&response, 50699).unwrap();
        println!("response is: {:?}", resp);
    }
}
