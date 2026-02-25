use std::collections::HashMap;

use common::{
    CT_TEXT_PLAIN, HttpHeader, HttpMethod, HttpProtocol, TE_CHUNKED, Url,
    http_writer::HttpBodyWriter,
    result::{HttpError, HttpResult},
};
use mini_runtime::{
    ConnTimeout,
    web::{
        client::{Client, ClientBuilder},
        conn::_Conn,
    },
};
use serde::Serialize;

use crate::response::ClientResponse;

pub struct ClientRequest {
    method: HttpMethod,
    url: Option<Url>,
    protocol: HttpProtocol,
    header: HttpHeader,
    timeout: ConnTimeout,
}

const DEFAULT_USER_AGENT: &str = "mini client/1.0";

impl ClientRequest {
    pub fn set_url(&mut self, url_str: impl AsRef<str>) -> HttpResult<&mut Self> {
        self.url.replace(Url::from(url_str)?);
        Ok(self)
    }

    pub fn set_protocol(&mut self, protocol: HttpProtocol) -> &mut Self {
        self.protocol = protocol;
        self
    }

    pub fn update_header(&mut self, mut updater: impl FnMut(&mut HttpHeader)) -> &mut Self {
        updater(&mut self.header);
        self
    }

    pub fn update_timeout(&mut self, mut updater: impl FnMut(&mut ConnTimeout)) -> &mut Self {
        updater(&mut self.timeout);
        self
    }

    pub async fn get(
        &mut self,
        params: HashMap<String, String>,
    ) -> HttpResult<ClientResponse<_Conn>> {
        if let Some(url) = &mut self.url {
            url.path.set_params(params);
        }

        self.send(HttpMethod::Get, &[]).await
    }

    pub async fn post(&mut self, body: &[u8]) -> HttpResult<ClientResponse<_Conn>> {
        self.send(HttpMethod::Post, body).await
    }

    pub async fn post_json<T: Serialize>(
        &mut self,
        value: &T,
    ) -> HttpResult<ClientResponse<_Conn>> {
        self.post(serde_json::to_string(value)?.as_bytes()).await
    }

    // 发送chunked请求
    pub async fn post_batch(&mut self) -> HttpResult<ChunkedTransport> {
        let client = self.connect(HttpMethod::Post)?;

        self.header.set_transfer_encoding(TE_CHUNKED.into());
        let mut body_writer = HttpBodyWriter::new(client.writer());
        body_writer.write_first_line(&self.first_line()?)?;
        body_writer.write_header(&self.header)?;
        body_writer.flush().await?;

        Ok(ChunkedTransport { client })
    }

    async fn send(&mut self, method: HttpMethod, body: &[u8]) -> HttpResult<ClientResponse<_Conn>> {
        let client = self.connect(method)?;
        self.header
            .set_content_length(body.len().to_string().into())
            .clear_transfer_encoding();
        if self.header.get_content_type().is_none() {
            self.header.set_content_type(CT_TEXT_PLAIN.into());
        }

        let mut body_writer = HttpBodyWriter::new(client.writer());
        body_writer.write_first_line(&self.first_line()?)?;
        body_writer.write_header(&self.header)?;
        body_writer.write_fix_length_body(body).await?;

        ClientResponse::new(client.reader()).await
    }

    fn connect(&mut self, method: HttpMethod) -> HttpResult<Client> {
        self.method = method;
        let url = self
            .url
            .as_ref()
            .ok_or(HttpError::InvalidUrl(common::result::InvalidUrl::Domain))?;
        if self.header.get_user_agent().is_none() {
            self.header.set_user_agent(DEFAULT_USER_AGENT.into());
        }

        Ok(
            ClientBuilder::new(&url.domain.domain, url.domain.port.unwrap_or(80))
                .update_timeout(|conn_timeout| {
                    *conn_timeout = self.timeout.clone();
                })
                .connect()?,
        )
    }

    fn first_line(&self) -> HttpResult<String> {
        if let Some(url) = &self.url {
            Ok(format!("{} {} {}", self.method, url.path, self.protocol))
        } else {
            Err(common::result::HttpError::InvalidUrl(
                common::result::InvalidUrl::Path,
            ))
        }
    }
}

impl Default for ClientRequest {
    fn default() -> Self {
        Self {
            method: HttpMethod::Get,
            url: None,
            protocol: HttpProtocol::default(),
            header: HttpHeader::default(),
            timeout: ConnTimeout::new(None),
        }
    }
}

pub struct ChunkedTransport {
    client: Client,
}

impl ChunkedTransport {
    pub async fn send_chunk(&mut self, body: &[u8]) -> HttpResult<()> {
        let mut body_writer = HttpBodyWriter::new(self.client.writer());
        body_writer.write_chunked_body(body).await
    }

    pub async fn wait_resp(&mut self) -> HttpResult<ClientResponse<_Conn>> {
        self.send_chunk(&[]).await?;
        ClientResponse::new(self.client.reader()).await
    }
}
