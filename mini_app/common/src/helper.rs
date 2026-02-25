use std::{fmt::Write, vec};

use mini_runtime::err_log;

use crate::result::HttpResult;

/// 字符串切分
pub struct BytesSplitter<'a> {
    raw: &'a [u8],
    sp: &'a [u8],
}

impl<'a> BytesSplitter<'a> {
    pub fn new(raw: &'a [u8], sp: &'a [u8]) -> Self {
        Self { raw, sp }
    }
}

impl<'a> Iterator for BytesSplitter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(offset) = memchr::memmem::find(self.raw, self.sp) {
            let item = &self.raw[..offset];
            self.raw = &self.raw[offset + self.sp.len()..];
            return Some(item);
        }

        if !self.raw.is_empty() {
            let item = self.raw;
            self.raw = &self.raw[self.raw.len()..];
            return Some(item);
        }
        None
    }
}

pub fn slice_to_str(raw: &[u8]) -> HttpResult<&str> {
    Ok(str::from_utf8(raw)?)
}

/// # Safety
///
/// 只对ascii字符串有效
/// 将snake格式字符串的首字母进行大写，保证header key的格式统一
#[allow(clippy::ptr_arg)]
pub unsafe fn first_letter_uppercase(mut s: String) -> String {
    let slice = unsafe { s.as_bytes_mut() };
    let mut letter_start = true;
    for b in slice {
        if letter_start {
            letter_start = false;
            *b = b.to_ascii_uppercase();
        } else if *b == b'-' {
            letter_start = true;
        }
    }

    s
}

#[allow(clippy::char_lit_as_u8)]
pub fn parse_query_string(s: &str) -> String {
    let bs = s.as_bytes();
    let len = bs.len();
    let mut i = 0;
    let mut origin_bytes = vec![];
    loop {
        if i >= len {
            break;
        }
        if bs[i] == '%' as u8 {
            if i + 2 < len {
                match u8::from_ascii_radix(&bs[i + 1..i + 3], 16) {
                    Ok(v) => {
                        origin_bytes.push(v);
                        i += 3;
                    }
                    _ => {
                        origin_bytes.push(bs[i]);
                        i += 1;
                    }
                }
            }
        } else {
            origin_bytes.push(bs[i]);
            i += 1;
        }
    }
    err_log!(String::from_utf8(origin_bytes), "parse query_string failed").unwrap_or_default()
}

pub fn encode_query_string(s: &str) -> String {
    #[inline]
    fn is_safe(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~'
    }

    #[inline]
    fn encode_utf8(c: char) -> String {
        let mut buf = [0u8; 4];
        let mut s = String::new();
        for b in c.encode_utf8(&mut buf).as_bytes() {
            let _ = s.write_fmt(format_args!("%{:02X}", b));
        }

        s
    }

    let mut encoded = String::new();
    for c in s.chars() {
        if is_safe(c) {
            let _ = encoded.write_char(c);
        } else {
            let _ = encoded.write_str(&encode_utf8(c));
        }
    }
    encoded
}

#[cfg(test)]
mod test {
    use crate::helper::{encode_query_string, first_letter_uppercase, parse_query_string};

    #[test]
    fn test_first_letter_uppercase() {
        let s = "content-type".to_string();
        println!("uppercase: {}", unsafe { first_letter_uppercase(s) });
    }

    #[rt_entry::rt_test]
    async fn test_parse_query_string() {
        let s = "a%E4%B8%80%E5%8F%AA%E7%89%B9%E7%AB%8B%E7%8B%AC%E8%A1%8C%E7%9A%84%E7%8C%AA";
        println!("origin string: {}", parse_query_string(s));

        let encoded = encode_query_string("a一只特立独行的猪");
        println!("encoded query_string: {}", encoded);

        assert_eq!(s, encoded)
    }
}
