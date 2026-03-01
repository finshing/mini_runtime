use std::time;

use cli::request::new_client;
use common::{CT_APPLICATION_JSON, HttpStatus, helper::slice_to_str, result::HttpResult};
#[cfg(feature = "mock")]
use mini_runtime::config::CRLF;
use mini_runtime::err_log;
use serde::{Deserialize, Serialize};

const DONE: &str = "[DONE]";

#[rt_entry::main(log_level = "debug")]
async fn main() -> HttpResult<()> {
    let body = RequestBody {
        model: "ep-20250310174249-mn2ts".to_owned(),
        stream: true,
        messages: vec![
            Message {
                role: "system".to_owned(),
                content: "你是人工智能助手.".to_owned(),
            },
            Message {
                role: "user".to_owned(),
                content: "你帮我查询下呗.".to_owned(),
            },
        ],
    };
    chat(&body).await
}

async fn chat(body: &RequestBody) -> HttpResult<()> {
    let mut client = new_client();
    let token = option_env!("API_TOKEN").ok_or("no API_TOKEN given")?;
    let response = client
        .set_url("http://ark.cn-beijing.volces.com/api/v3/chat/completions")?
        .update_header(|header| {
            header
                .set_content_type(CT_APPLICATION_JSON.into())
                .set_authorization(format!("Bearer {}", token).into());
        })
        .update_timeout(|timeout| {
            timeout
                .update_timeout(time::Duration::from_secs(60))
                .set_read_timeout(time::Duration::from_secs(2));
        })
        .post_json(body)
        .await;

    #[cfg(feature = "mock")]
    {
        println!("request header: {:?}", client.request_header());
        let body = client.request_body();
        println!(
            "request body({:?}): {:?}",
            body.as_ref().map(|b| b.len() - CRLF.len()),
            body.map(|b| slice_to_str(&b).map(|b| b.to_owned()))
        )
    }

    #[cfg(feature = "tcp")]
    {
        let response = response?;
        log::info!("chat response status: {}", response.status());
        if response.status() != &HttpStatus::Success {
            let body = response.body_reader().read().await?;
            return Err(common::result::HttpError::ResponseError(
                *response.status(),
                slice_to_str(&body)?.to_owned(),
            ));
        }

        if let Some(mut sse_reader) = response.sse_reader() {
            print!(">>> ");
            loop {
                let event = err_log!(sse_reader.read_event().await, "event read failed")?;
                if let Some(event) = event {
                    // println!(">>> {}", event);
                    let data = event.get_data();
                    if data == DONE {
                        break;
                    }
                    let event_body = event.get_json_data::<ResponseEvent>()?;
                    print!("{}", event_body.content());
                } else {
                    break;
                }
            }
        } else {
            let body = response.body_reader().read().await?;
            log::info!("get response: {}", slice_to_str(&body)?);
            return Ok(());
        }
    }

    Ok(())
}

#[derive(Serialize, Debug)]
struct RequestBody {
    model: String,
    stream: bool,
    messages: Vec<Message>,
}

#[derive(Serialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
struct ResponseEvent {
    model: String,
    choices: Vec<Choice>,
}

impl ResponseEvent {
    fn content(&self) -> String {
        self.choices
            .iter()
            .map(|c| c.delta.content.as_str())
            .collect::<Vec<_>>()
            .join("")
    }
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
struct Choice {
    delta: Delta,
    index: usize,
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
struct Delta {
    content: String,
    role: String,
}
