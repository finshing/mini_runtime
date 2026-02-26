use std::{collections::HashMap, time};

use cli::request::HttpClient;
use common::{
    CT_EVENT_STREAM,
    dto::{ArticalListReqBody, BadResponse, SSEContent},
    helper::slice_to_str,
    result::HttpResult,
};
use mini_runtime::{TimerRecord, err_log};

#[rt_entry::main]
async fn main() -> HttpResult<()> {
    let timer_record = TimerRecord::new();
    let mut client = HttpClient::default();
    // get_home(&mut client).await?;
    // post_article_list(&mut client).await?;
    completion(&mut client).await?;
    timer_record.info("client request");
    Ok(())
}

fn url(path: &str) -> String {
    format!(
        "{}:{}{}",
        common::config::HTTP_SERVER_IP,
        common::config::HTTP_SERVER_PORT,
        path
    )
}

#[allow(unused)]
async fn get_home(client: &mut HttpClient) -> HttpResult<()> {
    let response = client.set_url(url("/home"))?.get(None).await?;
    log::info!("get /home status: {}", response.status());
    let body = response.body_reader().read().await?;
    log::info!(
        "get /home body: (length: {})\n {}",
        body.len(),
        slice_to_str(&body)?
    );
    Ok(())
}

#[allow(unused)]
async fn post_article_list(client: &mut HttpClient) -> HttpResult<()> {
    let mut response = client.set_url(url("/article_list"))?.post(&[]).await?;
    log::info!("get /article_list status: {}", response.status());
    let body = response.json::<ArticalListReqBody>().await?;
    log::info!("get /home body:\n{:?}", body);
    Ok(())
}

#[allow(unused)]
async fn completion(client: &mut HttpClient) -> HttpResult<()> {
    let mut response = client
        .set_url(url("/completion"))?
        .update_timeout(|timeout| {
            timeout.update_timeout(time::Duration::from_secs(20));
        })
        .get(Some(HashMap::from_iter([(
            "article".to_owned(),
            "背影".to_owned(),
        )])))
        .await?;

    if response.header().get_content_type() == Some(CT_EVENT_STREAM) {
        let mut sse_reader = response.sse_reader().unwrap();
        loop {
            match err_log!(sse_reader.read_event().await, "sse read_event failed")? {
                Some(event) => {
                    if let Ok(content) = err_log!(
                        event.get_json_data::<SSEContent>(),
                        "parse sse content failed"
                    ) {
                        print!("{}", content.content);
                    }
                }
                None => {
                    log::info!("no more message");
                    break;
                }
            }
        }
    } else {
        let result = response.json::<BadResponse>().await?;
        log::info!("request failed with result: {:?}", result);
    }

    Ok(())
}
