use std::{env, fs, net::SocketAddr, path::Path};

use serde::Deserialize;
use tracing_subscriber::EnvFilter;
use url::Url;

use crate::{
    events::{EventFilter, EventNames},
    notification::BodyFieldExclusions,
};

#[derive(Debug, Clone)]
pub struct Settings {
    pub server: ServerSettings,
    pub logging: LoggingSettings,
    pub notification: NotificationSettings,
    pub gotify: GotifySettings,
    pub forgejo: ForgejoSettings,
}

#[derive(Debug, Clone)]
pub struct LoggingSettings {
    pub filter: String,
}

#[derive(Debug, Clone)]
pub struct NotificationSettings {
    pub body_exclude_fields: BodyFieldExclusions,
}

#[derive(Debug, Clone)]
pub struct ServerSettings {
    pub bind_addr: SocketAddr,
    pub webhook_path: String,
}

#[derive(Debug, Clone)]
pub struct GotifySettings {
    pub base_url: Url,
    pub app_token: String,
    pub priority: i32,
    pub title_prefix: String,
}

#[derive(Debug, Clone)]
pub struct ForgejoSettings {
    pub secret: Option<String>,
    pub filter: EventFilter,
}

#[derive(Debug, Default, Deserialize)]
struct FileSettings {
    server: Option<FileServer>,
    logging: Option<FileLogging>,
    notification: Option<FileNotification>,
    gotify: Option<FileGotify>,
    forgejo: Option<FileForgejo>,
}

#[derive(Debug, Default, Deserialize)]
struct FileLogging {
    level: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileNotification {
    body_exclude_fields: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
struct FileServer {
    bind_addr: Option<String>,
    webhook_path: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileGotify {
    base_url: Option<String>,
    app_token: Option<String>,
    priority: Option<i32>,
    title_prefix: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct FileForgejo {
    secret: Option<String>,
    events: Option<Vec<String>>,
}

#[derive(Debug)]
struct RawSettings {
    log_level: String,
    notification_body_exclude_fields: Vec<String>,
    bind_addr: String,
    webhook_path: String,
    gotify_base_url: Option<String>,
    gotify_app_token: Option<String>,
    gotify_priority: i32,
    gotify_title_prefix: String,
    forgejo_secret: Option<String>,
    forgejo_events: Vec<String>,
}

impl Settings {
    pub fn load() -> anyhow::Result<Self> {
        let mut raw = RawSettings::default();
        raw.merge_file()?;
        raw.merge_env();
        raw.validate()
    }
}

impl Default for RawSettings {
    fn default() -> Self {
        Self {
            log_level: "info".to_owned(),
            notification_body_exclude_fields: Vec::new(),
            bind_addr: "0.0.0.0:3000".to_owned(),
            webhook_path: "/webhook".to_owned(),
            gotify_base_url: None,
            gotify_app_token: None,
            gotify_priority: 5,
            gotify_title_prefix: "Forgejo".to_owned(),
            forgejo_secret: None,
            forgejo_events: vec!["action_run_failure".to_owned()],
        }
    }
}

impl RawSettings {
    fn merge_file(&mut self) -> anyhow::Result<()> {
        let path = env::var("APP_CONFIG").unwrap_or_else(|_| "config.toml".to_owned());
        if !Path::new(&path).exists() {
            return Ok(());
        }

        let file: FileSettings = toml::from_str(&fs::read_to_string(&path)?)?;
        if let Some(logging) = file.logging {
            assign(&mut self.log_level, logging.level);
        }
        if let Some(notification) = file.notification
            && let Some(body_exclude_fields) = notification.body_exclude_fields
        {
            self.notification_body_exclude_fields = body_exclude_fields;
        }
        if let Some(server) = file.server {
            assign(&mut self.bind_addr, server.bind_addr);
            assign(&mut self.webhook_path, server.webhook_path);
        }
        if let Some(gotify) = file.gotify {
            self.gotify_base_url = gotify.base_url.or(self.gotify_base_url.take());
            self.gotify_app_token = gotify.app_token.or(self.gotify_app_token.take());
            assign(&mut self.gotify_priority, gotify.priority);
            assign(&mut self.gotify_title_prefix, gotify.title_prefix);
        }
        if let Some(forgejo) = file.forgejo {
            self.forgejo_secret = forgejo.secret.or(self.forgejo_secret.take());
            if let Some(events) = forgejo.events {
                self.forgejo_events = events;
            }
        }
        Ok(())
    }

    fn merge_env(&mut self) {
        assign(
            &mut self.log_level,
            read_env(["RUST_LOG", "LOG_LEVEL", "APP_LOG_LEVEL"]),
        );
        if let Some(fields) = read_env(["NOTIFICATION_BODY_EXCLUDE_FIELDS"]) {
            self.notification_body_exclude_fields = split_list(&fields);
        }
        assign(
            &mut self.bind_addr,
            read_env(["BIND_ADDR", "APP_BIND_ADDR"]),
        );
        assign(
            &mut self.webhook_path,
            read_env(["WEBHOOK_PATH", "APP_WEBHOOK_PATH"]),
        );
        self.gotify_base_url = read_env(["GOTIFY_BASE_URL"]).or(self.gotify_base_url.take());
        self.gotify_app_token = read_env(["GOTIFY_APP_TOKEN"]).or(self.gotify_app_token.take());
        if let Some(priority) = read_env(["GOTIFY_PRIORITY"]) {
            self.gotify_priority = priority.parse().unwrap_or(self.gotify_priority);
        }
        assign(
            &mut self.gotify_title_prefix,
            read_env(["GOTIFY_TITLE_PREFIX"]),
        );
        self.forgejo_secret = read_env(["FORGEJO_SECRET"]).or(self.forgejo_secret.take());
        if let Some(events) = read_env(["FORGEJO_EVENTS"]) {
            self.forgejo_events = split_list(&events);
        }
    }

    fn validate(self) -> anyhow::Result<Settings> {
        EnvFilter::try_new(&self.log_level).map_err(|err| {
            anyhow::anyhow!("invalid log level/filter `{}`: {}", self.log_level, err)
        })?;
        let bind_addr = self.bind_addr.parse()?;
        let webhook_path = validate_path(self.webhook_path)?;
        let base_url = self
            .gotify_base_url
            .ok_or_else(|| anyhow::anyhow!("GOTIFY_BASE_URL is required"))?;
        let app_token = self
            .gotify_app_token
            .ok_or_else(|| anyhow::anyhow!("GOTIFY_APP_TOKEN is required"))?;
        let filter = EventFilter::new(EventNames::new(self.forgejo_events))?;
        let body_exclude_fields = BodyFieldExclusions::new(self.notification_body_exclude_fields)?;

        Ok(Settings {
            server: ServerSettings {
                bind_addr,
                webhook_path,
            },
            logging: LoggingSettings {
                filter: self.log_level,
            },
            notification: NotificationSettings {
                body_exclude_fields,
            },
            gotify: GotifySettings {
                base_url: Url::parse(base_url.trim_end_matches('/'))?,
                app_token,
                priority: self.gotify_priority,
                title_prefix: self.gotify_title_prefix,
            },
            forgejo: ForgejoSettings {
                secret: blank_to_none(self.forgejo_secret),
                filter,
            },
        })
    }
}

fn read_env<const N: usize>(names: [&str; N]) -> Option<String> {
    names.into_iter().find_map(|name| env::var(name).ok())
}

fn assign<T>(slot: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *slot = value;
    }
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().to_owned())
        .collect()
}

fn validate_path(path: String) -> anyhow::Result<String> {
    anyhow::ensure!(path.starts_with('/'), "WEBHOOK_PATH must start with /");
    anyhow::ensure!(path.len() > 1, "WEBHOOK_PATH must not be /");
    Ok(path)
}

fn blank_to_none(value: Option<String>) -> Option<String> {
    value.and_then(|value| (!value.trim().is_empty()).then_some(value))
}
