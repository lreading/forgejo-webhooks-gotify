use axum::http::HeaderMap;
use bytes::Bytes;
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;

use crate::notification::{BodyField, BodyFieldExclusions};

#[derive(Debug, Clone)]
pub struct WebhookPayload {
    pub event: String,
    pub event_type: Option<String>,
    pub delivery: Option<String>,
    pub body: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum WebhookError {
    #[error("missing X-Forgejo-Event header")]
    MissingEvent,
    #[error("missing X-Forgejo-Signature header")]
    MissingSignature,
    #[error("invalid X-Forgejo-Signature header")]
    InvalidSignature,
    #[error("webhook body is not valid JSON")]
    InvalidJson(#[from] serde_json::Error),
}

impl WebhookPayload {
    pub fn from_request(headers: &HeaderMap, body: &Bytes) -> Result<Self, WebhookError> {
        let event = header(headers, "x-forgejo-event")
            .or_else(|| header(headers, "x-gitea-event"))
            .ok_or(WebhookError::MissingEvent)?;
        let event_type = header(headers, "x-forgejo-event-type")
            .or_else(|| header(headers, "x-gitea-event-type"));
        let delivery =
            header(headers, "x-forgejo-delivery").or_else(|| header(headers, "x-gitea-delivery"));

        Ok(Self {
            event: event.to_owned(),
            event_type: event_type.map(str::to_owned),
            delivery: delivery.map(str::to_owned),
            body: serde_json::from_slice(body)?,
        })
    }

    pub fn title(&self, prefix: &str) -> String {
        let subject = self.repository().unwrap_or("unknown repository");
        let kind = self.event_type.as_deref().unwrap_or(&self.event);
        format!("{prefix}: {kind} in {subject}")
    }

    pub fn message(&self, exclusions: &BodyFieldExclusions) -> String {
        let mut lines = Vec::new();
        push_line(
            exclusions,
            &mut lines,
            BodyField::Event,
            self.event_type.as_deref().unwrap_or(&self.event),
        );
        push_optional_line(
            exclusions,
            &mut lines,
            BodyField::Repository,
            self.repository(),
        );
        push_optional_line(
            exclusions,
            &mut lines,
            BodyField::Action,
            self.body.get("action").and_then(Value::as_str),
        );
        push_optional_line(
            exclusions,
            &mut lines,
            BodyField::PriorStatus,
            str_at(&self.body, "/prior_status"),
        );
        push_optional_line(exclusions, &mut lines, BodyField::Ref, self.ref_name());
        push_optional_line(exclusions, &mut lines, BodyField::Commit, self.commit_sha());
        push_optional_line(exclusions, &mut lines, BodyField::Sender, self.sender());
        push_optional_line(exclusions, &mut lines, BodyField::Url, self.best_url());
        push_optional_line(
            exclusions,
            &mut lines,
            BodyField::Delivery,
            self.delivery.as_deref(),
        );
        lines.join("\n")
    }

    pub fn extras(&self) -> Value {
        let mut extras = json!({
            "client::display": { "contentType": "text/plain" },
            "forgejo::webhook": {
                "event": self.event,
                "event_type": self.event_type,
                "delivery": self.delivery,
                "payload": self.body,
            }
        });

        if let Some(url) = self.best_url() {
            extras["client::notification"] = json!({
                "click": { "url": url }
            });
        }

        extras
    }

    fn repository(&self) -> Option<&str> {
        str_at(&self.body, "/repository/full_name")
            .or_else(|| str_at(&self.body, "/repository/name"))
            .or_else(|| str_at(&self.body, "/run/repository/full_name"))
            .or_else(|| str_at(&self.body, "/run/repository/name"))
    }

    fn sender(&self) -> Option<&str> {
        str_at(&self.body, "/sender/login")
            .or_else(|| str_at(&self.body, "/sender/username"))
            .or_else(|| str_at(&self.body, "/run/trigger_user/login"))
            .or_else(|| str_at(&self.body, "/run/trigger_user/username"))
    }

    fn ref_name(&self) -> Option<&str> {
        str_at(&self.body, "/ref")
            .or_else(|| str_at(&self.body, "/run/prettyref"))
            .or_else(|| str_at(&self.body, "/pull_request/head/ref"))
    }

    fn commit_sha(&self) -> Option<&str> {
        str_at(&self.body, "/after")
            .or_else(|| str_at(&self.body, "/run/commit_sha"))
            .or_else(|| str_at(&self.body, "/pull_request/head/sha"))
    }

    fn best_url(&self) -> Option<&str> {
        str_at(&self.body, "/workflow_run/html_url")
            .or_else(|| str_at(&self.body, "/run/html_url"))
            .or_else(|| str_at(&self.body, "/compare_url"))
            .or_else(|| str_at(&self.body, "/pull_request/html_url"))
            .or_else(|| str_at(&self.body, "/issue/html_url"))
            .or_else(|| str_at(&self.body, "/release/html_url"))
            .or_else(|| str_at(&self.body, "/repository/html_url"))
            .or_else(|| str_at(&self.body, "/run/repository/html_url"))
    }
}

pub fn verify_signature(
    secret: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<(), WebhookError> {
    let signature = header(headers, "x-forgejo-signature")
        .or_else(|| header(headers, "x-gitea-signature"))
        .ok_or(WebhookError::MissingSignature)?;
    let expected = hex::decode(signature).map_err(|_| WebhookError::InvalidSignature)?;
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .map_err(|_| WebhookError::InvalidSignature)?;
    mac.update(body);
    mac.verify_slice(&expected)
        .map_err(|_| WebhookError::InvalidSignature)
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

fn str_at<'a>(value: &'a Value, pointer: &str) -> Option<&'a str> {
    value.pointer(pointer).and_then(Value::as_str)
}

fn push_line(
    exclusions: &BodyFieldExclusions,
    lines: &mut Vec<String>,
    field: BodyField,
    value: &str,
) {
    if !exclusions.includes(field) {
        lines.push(format!("{field}: {value}"));
    }
}

fn push_optional_line(
    exclusions: &BodyFieldExclusions,
    lines: &mut Vec<String>,
    field: BodyField,
    value: Option<&str>,
) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        push_line(exclusions, lines, field, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_action_run_payload_fields() {
        let exclusions = BodyFieldExclusions::new(Vec::new()).unwrap();
        let payload = WebhookPayload {
            event: "action_run_failure".into(),
            event_type: Some("action_run_failure".into()),
            delivery: Some("delivery-id".into()),
            body: json!({
                "action": "failure",
                "prior_status": "running",
                "run": {
                    "commit_sha": "bd13e1f96bbd67777c8fcbf73d83ece9d6a31f61",
                    "html_url": "https://git.example.com/leo/webhook-test/actions/runs/4",
                    "prettyref": "main",
                    "repository": {
                        "full_name": "leo/webhook-test",
                        "html_url": "https://git.example.com/leo/webhook-test"
                    },
                    "trigger_user": {
                        "login": "leo"
                    }
                }
            }),
        };

        assert_eq!(
            payload.title("Forgejo"),
            "Forgejo: action_run_failure in leo/webhook-test"
        );
        assert_eq!(
            payload.message(&exclusions),
            [
                "event: action_run_failure",
                "repository: leo/webhook-test",
                "action: failure",
                "prior_status: running",
                "ref: main",
                "commit: bd13e1f96bbd67777c8fcbf73d83ece9d6a31f61",
                "sender: leo",
                "url: https://git.example.com/leo/webhook-test/actions/runs/4",
                "delivery: delivery-id",
            ]
            .join("\n")
        );
    }

    #[test]
    fn adds_click_url_when_payload_has_url() {
        let payload = WebhookPayload {
            event: "action_run_failure".into(),
            event_type: Some("action_run_failure".into()),
            delivery: Some("delivery-id".into()),
            body: json!({
                "run": {
                    "html_url": "https://git.example.com/leo/webhook-test/actions/runs/4"
                }
            }),
        };

        assert_eq!(
            payload.extras()["client::notification"]["click"]["url"],
            "https://git.example.com/leo/webhook-test/actions/runs/4"
        );
    }

    #[test]
    fn omits_click_url_when_payload_has_no_url() {
        let payload = WebhookPayload {
            event: "repository".into(),
            event_type: Some("repository".into()),
            delivery: None,
            body: json!({ "action": "created" }),
        };

        assert!(payload.extras().get("client::notification").is_none());
    }

    #[test]
    fn excludes_configured_body_fields() {
        let exclusions = BodyFieldExclusions::new(vec![
            "prior_status".into(),
            "ref".into(),
            "commit".into(),
            "sender".into(),
        ])
        .unwrap();
        let payload = WebhookPayload {
            event: "action_run_failure".into(),
            event_type: Some("action_run_failure".into()),
            delivery: None,
            body: json!({
                "action": "failure",
                "prior_status": "running",
                "run": {
                    "commit_sha": "bd13e1f96bbd67777c8fcbf73d83ece9d6a31f61",
                    "html_url": "https://git.example.com/leo/webhook-test/actions/runs/4",
                    "prettyref": "main",
                    "repository": { "full_name": "leo/webhook-test" },
                    "trigger_user": { "login": "leo" }
                }
            }),
        };

        assert_eq!(
            payload.message(&exclusions),
            [
                "event: action_run_failure",
                "repository: leo/webhook-test",
                "action: failure",
                "url: https://git.example.com/leo/webhook-test/actions/runs/4",
            ]
            .join("\n")
        );
    }

    #[test]
    fn renders_documented_push_payload_fields() {
        let exclusions = BodyFieldExclusions::new(Vec::new()).unwrap();
        let payload = WebhookPayload {
            event: "push".into(),
            event_type: Some("push".into()),
            delivery: Some("delivery-id".into()),
            body: json!({
                "ref": "refs/heads/develop",
                "after": "bffeb74224043ba2feb48d137756c8a9331c449a",
                "compare_url": "https://git.example.com/forgejo/webhooks/compare/range",
                "repository": {
                    "full_name": "forgejo/webhooks",
                    "html_url": "https://git.example.com/forgejo/webhooks"
                },
                "sender": {
                    "login": "forgejo"
                }
            }),
        };

        assert_eq!(
            payload.title("Forgejo"),
            "Forgejo: push in forgejo/webhooks"
        );
        assert_eq!(
            payload.message(&exclusions),
            [
                "event: push",
                "repository: forgejo/webhooks",
                "ref: refs/heads/develop",
                "commit: bffeb74224043ba2feb48d137756c8a9331c449a",
                "sender: forgejo",
                "url: https://git.example.com/forgejo/webhooks/compare/range",
                "delivery: delivery-id",
            ]
            .join("\n")
        );
    }
}
