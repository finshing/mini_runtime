use mini_runtime::web::client::{Client, ClientBuilder};

use crate::{receive, request::Request, response::Response, result::RedisResult, send};

pub struct RedisClient {
    client: Client,
}

impl RedisClient {
    pub fn new(ip: &str, port: usize) -> RedisResult<Self> {
        Ok(Self {
            client: ClientBuilder::new(ip, port).connect()?,
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
