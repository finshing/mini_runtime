// https://github.com/rust-lang/rust/pull/142704
// concat_ident!在1.90版本被移除，可以在之前的版本使用
#![feature(macro_metavar_expr_concat)]
#![feature(int_from_ascii)]

use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::{Display, Formatter},
    sync::Once,
};

use mini_runtime::BoxedFutureWithError;

use crate::{
    helper::{BytesSplitter, encode_query_string, parse_query_string, slice_to_str},
    result::{HttpError, HttpResult, InvalidUrl},
};

pub mod config;
pub mod dto;
pub mod helper;
pub mod http_reader;
pub mod http_writer;
pub mod macros;
pub mod result;
pub mod sse_proto;

pub const LFLF: &str = "\n\n";
pub const CRLF: &str = "\r\n";
pub const CRLFCRLF: &str = "\r\n\r\n";
pub const DEFAULT_HTTP_PROTOCOL: &str = "HTTP/1.1";

// Contenty-Type的常见值
pub const CT_APPLICATION_JSON: &str = "application/json";
pub const CT_TEXT_PLAIN: &str = "text/plain";
pub const CT_TEXT_HTML: &str = "text/html";
pub const CT_EVENT_STREAM: &str = "text/event-stream";
pub const CT_FORM_URLENCODED: &str = "application/x-www-form-urlencoded";

// Accept的常见值
pub const ACCEPT_ALL: &str = "*/*";

// Transport-Encode的常见值
pub const TE_CHUNKED: &str = "chunked";

pub type HttpBoxedFuture<'a, T> = BoxedFutureWithError<'a, T, HttpError>;

// 初始化部分常见的状态码
init_http_status! {
    (Success, 200, "success"),

    (MovedPermanently, 301, "moved permanently"),

    (BadRequest, 400, "bad request"),
    (Forbidden, 403, "forbidden"),
    (NotFound, 404, "not found"),
    (MethodNotAllowed, 405, "method not allowed"),

    (InternalServerError, 500, "internal server error"),
}

// 初始化部分常见的请求方法
init_http_method! {
    (Get, "GET"),
    (Post, "POST"),
    (Head, "HEAD"),
    (Options, "OPTIONS"),
}

// 初始化部分常见的请求头
init_http_header! {
    (host, "Host"),
    (user_agent, "User-Agent"),
    (accept, "Accept"),
    (accept_encoding, "Accept-Encoding"),
    (accept_language, "Accept-Language"),
    (content_type, "Content-Type"),
    (content_length, "Content-Length"),
    (transfer_encoding, "Transfer-Encoding"),
    (authorization, "Authorization"),
    (cache_control, "Cache-Control"),
    (connection, "Connection"),
    (location, "Location"),
}

impl HttpHeader {
    pub fn content_length(&self) -> Option<usize> {
        self.get_content_length()
            .map(|len| len.parse::<usize>().unwrap_or(0))
    }
}

// 协议版本
#[derive(Clone, Debug)]
pub struct HttpProtocol(String, String);

impl HttpProtocol {
    pub fn new(s: &str) -> HttpResult<Self> {
        let mut it = s.splitn(2, "/");
        let proto = it
            .next()
            .ok_or_else(|| format!("proto not exist in {}", s))?;
        let version = it
            .next()
            .ok_or_else(|| format!("version not exist in {}", s))?;

        Ok(Self(proto.to_owned(), version.to_owned()))
    }
}

impl Display for HttpProtocol {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.0, self.1)
    }
}

impl Default for HttpProtocol {
    fn default() -> Self {
        Self::new(DEFAULT_HTTP_PROTOCOL).unwrap()
    }
}

#[derive(Debug)]
pub struct Url {
    pub scheme: HttpScheme,
    pub domain: Domain,
    pub path: UrlPath,
}

impl Url {
    pub fn from<T: AsRef<str>>(url_string: T) -> HttpResult<Self> {
        let mut scheme_splitter =
            BytesSplitter::new(url_string.as_ref().as_bytes(), "://".as_bytes())
                .map(slice_to_str)
                .map(|s| s.map(String::from))
                .collect::<Vec<_>>();

        let (scheme, url) = if scheme_splitter.len() == 1 {
            (
                HttpScheme::from("")?,
                std::mem::replace(&mut scheme_splitter[0], Ok(String::new()))?,
            )
        } else if scheme_splitter.len() == 2 {
            (
                HttpScheme::from(std::mem::replace(
                    &mut scheme_splitter[0],
                    Ok(String::new()),
                )?)?,
                std::mem::replace(&mut scheme_splitter[1], Ok(String::new()))?,
            )
        } else {
            return Err(HttpError::InvalidUrl(InvalidUrl::Path));
        };

        let mut url_splitter = url.splitn(2, '/');
        let domain = url_splitter.next().ok_or("domain not exist")?;
        let path = url_splitter
            .next()
            .map(|p| format!("/{}", p))
            .unwrap_or(String::new())
            .into();

        Ok(Self {
            scheme,
            domain: Domain::from(domain)?,
            path,
        })
    }
}

// 请求协议
#[derive(Debug)]
pub enum HttpScheme {
    HTTP,
    HTTPS,
}

impl HttpScheme {
    pub fn from<T: AsRef<str>>(scheme: T) -> HttpResult<Self> {
        match scheme.as_ref() {
            "https" => Ok(Self::HTTPS),
            "http" => Ok(Self::HTTP),
            "" => Ok(Self::HTTP),
            _ => Err(HttpError::InvalidUrl(result::InvalidUrl::Schema)),
        }
    }
}

impl Display for HttpScheme {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HTTP => write!(f, "http://"),
            Self::HTTPS => write!(f, "https://"),
        }
    }
}

// 请求地址
#[derive(Debug)]
pub struct Domain {
    pub host: String,
    pub port: Option<u16>,
}

impl Domain {
    pub fn from(s: &str) -> HttpResult<Self> {
        let mut domain_splitter = BytesSplitter::new(s.as_bytes(), ":".as_bytes());
        let domain = slice_to_str(domain_splitter.next().ok_or("domain is empty")?)?.to_owned();
        let port = domain_splitter
            .next()
            .map(slice_to_str)
            .transpose()?
            .map(|p| p.parse::<u16>())
            .transpose()?;

        Ok(Self { host: domain, port })
    }
}

// 请求路径
#[derive(Debug)]
pub struct UrlPath {
    pub path: String,

    #[allow(unused)]
    query_string: String,
    once: Once,
    _params: RefCell<HashMap<String, String>>,
}

impl UrlPath {
    pub fn new(url: &str) -> Self {
        let mut result = url.splitn(2, "?");

        let path = result.next().unwrap_or("").to_owned();
        let query_string = result.next().unwrap_or("").to_owned();

        Self {
            path,
            query_string,
            once: Once::new(),
            _params: RefCell::new(HashMap::new()),
        }
    }

    pub fn set_params(&mut self, params: HashMap<String, String>) {
        let mut _params = self._params.borrow_mut();
        for (k, v) in params.into_iter() {
            _params.insert(k, v);
        }
    }

    pub fn get_param(&self, key: &str) -> Option<String> {
        fn insert_kv(ps: &mut HashMap<String, String>, s: &[u8]) {
            let mut splitter = BytesSplitter::new(s, "=".as_bytes()).map(slice_to_str);
            if let Some(Ok(key)) = splitter.next()
                && let Some(Ok(value)) = splitter.next()
            {
                ps.insert(key.to_owned(), parse_query_string(value));
            }
        }

        // 只解析一次query_string
        self.once.call_once(|| {
            let mut ps = self._params.borrow_mut();
            let splitter = BytesSplitter::new(self.query_string.as_bytes(), "&".as_bytes());
            for s in splitter {
                insert_kv(&mut ps, s);
            }
        });

        let ps = self._params.borrow();
        ps.get(key).map(|v| v.to_owned())
    }
}

impl Display for UrlPath {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path)?;
        let _params = self._params.borrow();
        if !_params.is_empty() {
            write!(f, "?")?;
            let mut write_flag = false;
            for (k, v) in _params.iter() {
                if write_flag {
                    write!(f, "&")?;
                }
                write!(f, "{}={}", encode_query_string(k), encode_query_string(v))?;
                write_flag = true;
            }
        }
        Ok(())
    }
}

impl<T: AsRef<str>> From<T> for UrlPath {
    fn from(value: T) -> Self {
        UrlPath::new(value.as_ref())
    }
}

#[cfg(test)]
mod test {
    use crate::Url;

    #[test]
    fn test_url_parse() {
        for url_str in [
            "http://localhost/index?a=1",
            "localhost:8991/path/to/home?a=aba",
            "https://localhost/completion",
        ] {
            println!("origin url: {}", url_str);
            let url = Url::from(url_str).unwrap();
            println!(
                "scheme: {:?} | domain: {:?} | path: {} | query_string: {}(query_param of a: {:?})",
                url.scheme,
                url.domain,
                url.path.path,
                url.path.query_string,
                url.path.get_param("a")
            );
        }
    }
}
