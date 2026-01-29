use mini_runtime::web::client::Client;

use crate::{receive, request::Request, response::Response, result::RedisResult, send};

pub struct RedisClient {
    client: Client,
}

impl RedisClient {
    pub fn new(ip: &str, port: usize) -> RedisResult<Self> {
        Ok(Self {
            client: Client::connect(ip, port)?,
        })
    }

    pub async fn cmd(&self, cmd: &Request) -> RedisResult<Response> {
        self.send(cmd).await?;
        self.receive().await
    }

    async fn send(&self, cmd: &Request) -> RedisResult<()> {
        let req_body = serde_json::to_string(cmd)?;
        send(self.client.writer(), req_body.as_bytes()).await
    }

    async fn receive(&self) -> RedisResult<Response> {
        let body = receive(self.client.reader()).await?;
        Ok(serde_json::from_slice(&body)?)
    }
}
