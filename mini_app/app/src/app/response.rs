use std::{string::IntoChars, time};

use serde::Serialize;

#[derive(Serialize)]
pub struct BadResponse {
    reason: String,
}

impl BadResponse {
    pub fn new(reason: String) -> Self {
        Self { reason }
    }
}

#[derive(Serialize, Debug)]
pub struct SSEContent {
    content: String,
    is_stop: bool,
}

impl SSEContent {
    pub fn resume(content: String) -> Self {
        Self {
            content,
            is_stop: false,
        }
    }

    pub fn stop() -> Self {
        Self {
            content: String::new(),
            is_stop: true,
        }
    }
}

pub(crate) struct TextRandomSplitter {
    content: IntoChars,
    t: time::Instant,
}

impl TextRandomSplitter {
    pub(crate) fn new(content: String) -> Self {
        Self {
            content: content.into_chars(),
            t: time::Instant::now(),
        }
    }
}

impl Iterator for TextRandomSplitter {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let size = (time::Instant::now().duration_since(self.t).subsec_micros() % 7) as usize;
        let mut s = String::new();
        for _ in 0..size + 3 {
            if let Some(c) = self.content.next() {
                s.push(c);
            } else {
                break;
            }
        }
        if s.is_empty() { None } else { Some(s) }
    }
}

#[cfg(test)]
mod tests {
    use std::time;

    use mini_runtime::sleep;

    use crate::{app::response::TextRandomSplitter, helper::load_file};

    #[rt_entry::rt_test]
    async fn test_text_splitter() {
        let content = load_file("./static/text/one_dream.txt").unwrap();
        let splitter = TextRandomSplitter::new(content);

        for chunk in splitter {
            print!("{}", chunk);
            sleep(time::Duration::from_millis(50)).await;
        }
    }
}
