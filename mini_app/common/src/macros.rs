/// 初始化http状态
/// 支持usize和http状态之间的相互抓换
#[macro_export]
macro_rules! init_http_status {
    (
        $( ($name:ident, $code:expr, $desc:expr), )+
    ) => {
        $(
            #[allow(non_upper_case_globals)]
            const $name: &str = $desc;
        )+

        #[derive(Clone, Copy, Debug)]
        pub enum HttpStatus {
            $( $name, )+
        }

        impl HttpStatus {
            pub fn code(&self) -> usize {
                match self {
                    $( Self::$name => $code, ) +
                }
            }

            pub fn description(&self) -> &'static str {
                match self {
                    $( Self::$name => $name, )+
                }
            }
        }

        impl From<usize> for HttpStatus {
            fn from(code: usize) -> Self {
                match code {
                    $(
                        $code => Self::$name,
                    )+
                    _ => Self::BadRequest,
                }
            }
        }

        impl std::fmt::Display for HttpStatus {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$name => write!(f, "{} {}", $code, $desc), )+
                }
            }
        }
    };
}

/// 初始化http请求方法
/// 支持HttpMethod和&str之间的相互转换
#[macro_export]
macro_rules! init_http_method {
    (
        $( ($method:ident, $val:expr), )+
    ) => {
        #[derive(Debug, Clone, Copy)]
        pub enum HttpMethod {
            $( $method, )+
        }

        impl <T: AsRef<str>> From<T> for HttpMethod {
            fn from(s: T) -> Self {
                match s.as_ref().to_uppercase().as_str() {
                    $( $val => Self::$method, )+
                    _ => Self::Get
                }
            }
        }

        impl std::fmt::Display for HttpMethod {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $( Self::$method => write!(f, "{}", $val), )+
                }
            }
        }
    };
}

/// http请求头初始化
/// 支持通过get_{header_name}()的方式获取{header_name}对应的请求头
/// 支持通过set_{header_name}()的方式设置{header_name}对应的请求头
#[macro_export]
macro_rules! init_http_header {
    (
        $( ($header_name:ident, $key:expr), )+
    ) => {
        #[derive(Default, Debug)]
        pub struct HttpHeader {
            headers: std::collections::HashMap<String, String>,
        }

        impl HttpHeader {
            pub fn set(&mut self, key: std::borrow::Cow<'_, str>, val: std::borrow::Cow<'_, str>) -> &mut Self {
                self.headers.insert(
                    unsafe { $crate::helper::first_letter_uppercase(key.into_owned()) },
                    val.into_owned(),
                );
                self
            }

            pub fn get(&self, key: &str) -> Option<&str> {
                self.headers.get(key).map(String::as_str)
            }

            $(
            pub fn ${concat(set_, $header_name)}(&mut self, value: std::borrow::Cow<'_, str>) -> &mut Self {
                self.set(std::borrow::Cow::from($key), value)
            }

            pub fn ${concat(get_, $header_name)}(&self) -> Option<&str> {
                if let Some(v) = self.get($key) {
                    return Some(v);
                }
                None
            }

            pub fn ${concat(clear_, $header_name)}(&mut self) -> &mut Self {
                self.headers.remove($key);
                self
            }
            )+
        }

        impl std::ops::Deref for HttpHeader {
            type Target = std::collections::HashMap<String, String>;

            fn deref(&self) -> &Self::Target {
                &self.headers
            }
        }
    };
}

/// http解析报错
#[macro_export]
macro_rules! http_parse_error {
    (
        $( ($ty:ty, $desc:expr), )+
    ) => {
        $(
        impl From<$ty> for HttpError {
            fn from(e: $ty) -> Self {
                Self::ParseError(format!("{} - {:?}", $desc, e).to_owned())
            }
        }
        )+
    }
}

#[macro_export]
macro_rules! allow_body_write {
    ($cur_state:expr, $expect_state:pat, $err:expr) => {
        if !matches!($cur_state, $expect_state) {
            return Err($crate::result::HttpError::InvalidBody(
                $crate::result::InvalidBody::WriteNotAllowed($err.to_owned()),
            ));
        }
    };
}
