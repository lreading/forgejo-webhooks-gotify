use std::time::Duration;

use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

use crate::{
    config::Settings,
    forgejo::{self, WebhookError, WebhookPayload},
    gotify::GotifyClient,
};

#[derive(Clone)]
pub struct AppState {
    settings: Settings,
    gotify: GotifyClient,
}

impl AppState {
    pub fn new(settings: Settings, gotify: GotifyClient) -> Self {
        Self { settings, gotify }
    }
}

pub async fn serve(state: AppState) -> anyhow::Result<()> {
    let bind_addr = state.settings.server.bind_addr;
    let webhook_path = state.settings.server.webhook_path.clone();
    let app = Router::new()
        .route("/health", get(health))
        .route(&webhook_path, post(webhook))
        .with_state(state);

    let listener = TcpListener::bind(bind_addr).await?;
    info!(%bind_addr, %webhook_path, "listening for Forgejo webhooks");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

async fn webhook(State(state): State<AppState>, headers: HeaderMap, body: Bytes) -> Response {
    if let Some(secret) = &state.settings.forgejo.secret
        && let Err(err) = forgejo::verify_signature(secret, &headers, &body)
    {
        warn!(%err, "rejected webhook signature");
        return status_for_webhook_error(&err).into_response();
    }

    let payload = match WebhookPayload::from_request(&headers, &body) {
        Ok(payload) => payload,
        Err(err) => {
            warn!(%err, "rejected webhook request");
            return status_for_webhook_error(&err).into_response();
        }
    };

    if !state
        .settings
        .forgejo
        .filter
        .is_known_incoming(&payload.event, payload.event_type.as_deref())
    {
        warn!(
            event = %payload.event,
            event_type = ?payload.event_type,
            "received unknown Forgejo webhook event"
        );
    }

    if !state
        .settings
        .forgejo
        .filter
        .is_subscribed(&payload.event, payload.event_type.as_deref())
    {
        debug!(
            event = %payload.event,
            event_type = ?payload.event_type,
            "ignored unsubscribed Forgejo webhook event"
        );
        return StatusCode::NO_CONTENT.into_response();
    }

    debug!(
        event = %payload.event,
        event_type = ?payload.event_type,
        delivery = ?payload.delivery,
        payload = %payload.body,
        "received subscribed Forgejo webhook payload"
    );

    let title = payload.title(&state.settings.gotify.title_prefix);
    let message = payload.message(&state.settings.notification.body_exclude_fields);
    debug!(%title, %message, "rendered Gotify notification");
    match state.gotify.send(title, message, payload.extras()).await {
        Ok(()) => {
            info!(
                event = %payload.event,
                event_type = ?payload.event_type,
                delivery = ?payload.delivery,
                "forwarded Forgejo webhook to Gotify"
            );
            StatusCode::ACCEPTED.into_response()
        }
        Err(err) => {
            error!(%err, "failed to send Gotify message");
            (StatusCode::BAD_GATEWAY, "failed to send Gotify message").into_response()
        }
    }
}

fn status_for_webhook_error(err: &WebhookError) -> StatusCode {
    match err {
        WebhookError::MissingSignature | WebhookError::InvalidSignature => StatusCode::UNAUTHORIZED,
        WebhookError::MissingEvent | WebhookError::InvalidJson(_) => StatusCode::BAD_REQUEST,
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    info!("shutdown signal received");
}
