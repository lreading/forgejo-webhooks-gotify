use reqwest::StatusCode;
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::config::GotifySettings;

#[derive(Debug, Clone)]
pub struct GotifyClient {
    client: reqwest::Client,
    message_url: Url,
    app_token: String,
    priority: i32,
}

#[derive(Debug, Serialize)]
pub struct GotifyMessage {
    title: String,
    message: String,
    priority: i32,
    extras: Value,
}

#[derive(Debug, Error)]
pub enum GotifyError {
    #[error("Gotify request failed")]
    Request(#[from] reqwest::Error),
    #[error("Gotify returned {status}: {body}")]
    Status { status: StatusCode, body: String },
}

impl GotifyClient {
    pub fn new(settings: &GotifySettings) -> Self {
        Self {
            client: reqwest::Client::new(),
            message_url: message_url(&settings.base_url),
            app_token: settings.app_token.clone(),
            priority: settings.priority,
        }
    }

    pub async fn send(
        &self,
        title: String,
        message: String,
        extras: Value,
    ) -> Result<(), GotifyError> {
        let payload = GotifyMessage {
            title,
            message,
            priority: self.priority,
            extras,
        };
        let response = self
            .client
            .post(self.message_url.clone())
            .query(&[("token", &self.app_token)])
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(GotifyError::Status {
                status: response.status(),
                body: response.text().await.unwrap_or_default(),
            });
        }
        Ok(())
    }
}

fn message_url(base_url: &Url) -> Url {
    let mut url = base_url.clone();
    let path = url.path().trim_end_matches('/');
    url.set_path(&format!("{path}/message"));
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_message_under_subpath() {
        let url = Url::parse("https://example.com/gotify").unwrap();
        assert_eq!(
            message_url(&url).as_str(),
            "https://example.com/gotify/message"
        );
    }
}
