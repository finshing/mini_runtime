use std::{
    borrow::Cow,
    fmt::{Display, Write},
};

use mini_runtime::err_log;
use serde::Serialize;

use crate::{
    helper::{BytesSplitter, slice_to_str},
    result::HttpResult,
};

const FIELD_DATA: Field = Field::new(FieldName::Data);
const FIELD_EVENT: Field = Field::new(FieldName::Event);
const FIELD_ID: Field = Field::new(FieldName::Id);
const FIELD_RETRY: Field = Field::new(FieldName::Retry);

const LINE_SPLITTER: &str = "\n";

#[derive(Debug)]
pub struct SSEProto {
    data: Field,
    event: Field,
    id: Field,
    retry: Field,
}

impl SSEProto {
    pub fn new(data: Cow<'_, str>) -> Self {
        let mut proto = Self::default();
        proto.set_data(data);

        proto
    }

    pub fn set_data(&mut self, data: Cow<'_, str>) -> &mut Self {
        self.data.set_value(data.into_owned());
        self
    }

    pub fn set_event(&mut self, data: Cow<'_, str>) -> &mut Self {
        self.event.set_value(data.into_owned());
        self
    }

    pub fn set_id(&mut self, data: Cow<'_, str>) -> &mut Self {
        self.id.set_value(data.into_owned());
        self
    }

    pub fn set_retry(&mut self, data: Cow<'_, str>) -> &mut Self {
        self.retry.set_value(data.into_owned());
        self
    }

    // 解析事件字段
    pub fn deserialize(value: &str) -> HttpResult<SSEProto> {
        let mut sse_proto = Self::default();
        let mut data = String::new();
        let splitter = BytesSplitter::new(value.as_bytes(), LINE_SPLITTER.as_bytes());
        for field_line in splitter {
            if let Ok(field) = err_log!(Field::from(field_line)) {
                match field.name {
                    FieldName::Data => {
                        _ = err_log!(data.write_str(field.value.as_str()));
                    }
                    FieldName::Event => sse_proto.event = field,
                    FieldName::Id => sse_proto.id = field,
                    FieldName::Retry => sse_proto.retry = field,
                }
            }
        }
        sse_proto.data.set_value(data);

        Ok(sse_proto)
    }
}

impl<T: Serialize> From<T> for SSEProto {
    fn from(value: T) -> Self {
        Self::new(serde_json::to_string(&value).unwrap_or_default().into())
    }
}

impl Default for SSEProto {
    fn default() -> Self {
        Self {
            data: FIELD_DATA,
            event: FIELD_EVENT,
            id: FIELD_ID,
            retry: FIELD_RETRY,
        }
    }
}

impl Display for SSEProto {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 以空行结尾表示事件完成
        write!(
            f,
            "{}{}{}{}{}",
            self.data, self.event, self.id, self.retry, LINE_SPLITTER
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FieldName {
    Data,
    Event,
    Id,
    Retry,
}

impl FieldName {
    fn from(value: &str) -> HttpResult<Self> {
        let value = value.to_lowercase();
        match value.as_str() {
            "data" => Ok(Self::Data),
            "event" => Ok(Self::Event),
            "id" => Ok(Self::Id),
            "retry" => Ok(Self::Retry),
            _ => Err(crate::result::HttpError::ParseError(format!(
                "unrecognized sse filed: {value}"
            ))),
        }
    }
}

impl Display for FieldName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Data => write!(f, "data"),
            Self::Event => write!(f, "event"),
            Self::Id => write!(f, "id"),
            Self::Retry => write!(f, "retry"),
        }
    }
}

#[derive(Debug)]
pub struct Field {
    name: FieldName,
    value: String,
}

impl Field {
    const fn new(name: FieldName) -> Self {
        Self {
            name,
            value: String::new(),
        }
    }

    fn set_value(&mut self, value: String) {
        self.value = value;
    }
}

impl Field {
    fn from(value: &[u8]) -> HttpResult<Self> {
        let mut splitter = BytesSplitter::new(value, ":".as_bytes());
        let name = splitter
            .next()
            .ok_or("lack sse field name")
            .map(slice_to_str)??;
        let value = splitter
            .next()
            .ok_or("lack sse field value")
            .map(slice_to_str)??;

        let field_name = FieldName::from(name)?;
        Ok(Field {
            name: field_name,
            value: value.trim_start().to_owned(),
        })
    }
}

/// 根据SSE协议要求：
/// - 各个字段之间通过LF、CR或者CRLF分隔。这里选择通过LF分隔
/// - data中的\n需要分隔为多行data字段
/// - 最后以一个空行作为整体事件的结束标识
impl Display for Field {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.value.is_empty() {
            for value in self.value.split(LINE_SPLITTER) {
                write!(f, "{}: {}{}", self.name, value, LINE_SPLITTER)?
            }
            Ok(())
        } else {
            write!(f, "")
        }
    }
}

#[cfg(test)]
mod test {
    use crate::sse_proto::SSEProto;

    #[test]
    fn test_sse_serialize() {
        let mut sse_proto = SSEProto::default();
        sse_proto
            .set_data("hello\nmini runtime server".into())
            .set_event("content".into())
            .set_id("1".into());

        println!("sse serialize:\n{}", sse_proto);
    }

    #[test]
    fn test_sse_deserialize() {
        let value = r#"data: what if
data: I lost
data: my way
event: thought
tag: invalid_tag
id:
retry
"#;
        let sse_proto = SSEProto::deserialize(value);
        println!("sse_proto: {:?}", sse_proto);
    }
}
