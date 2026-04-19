# forgejo-actions-failed-webhook-gotify

Listen for Forgejo v15+ webhooks and forward subscribed events to Gotify.

The default subscription is `action_run_failure`, intended for instance-wide Forgejo webhooks.

## Run

```sh
docker run --rm -p 3000:3000 \
  -e GOTIFY_BASE_URL=https://gotify.example.com \
  -e GOTIFY_APP_TOKEN=replace-me \
  ghcr.io/lreading/forgejo-webhooks-gotify:latest
```

Use a config file instead:

```sh
docker run --rm -p 3000:3000 \
  -v ./config.toml:/config/config.toml:ro \
  -e APP_CONFIG=/config/config.toml \
  ghcr.io/lreading/forgejo-webhooks-gotify:latest
```

Build locally:

```sh
docker build -t forgejo-webhooks-gotify .
```

## Configuration

The app reads `config.toml` by default. Set `APP_CONFIG` to use another file. Environment variables override file settings.

| TOML setting | Environment variable | Required | Default | Description |
| --- | --- | --- | --- | --- |
| `logging.level` | `RUST_LOG`, `LOG_LEVEL`, `APP_LOG_LEVEL` | No | `info` | Log level or tracing filter. |
| `notification.body_exclude_fields` | `NOTIFICATION_BODY_EXCLUDE_FIELDS` | No | `[]` | Body fields to hide. |
| `server.bind_addr` | `BIND_ADDR`, `APP_BIND_ADDR` | No | `0.0.0.0:3000` | Listen address and port. |
| `server.webhook_path` | `WEBHOOK_PATH`, `APP_WEBHOOK_PATH` | No | `/webhook` | Webhook route path. |
| `gotify.base_url` | `GOTIFY_BASE_URL` | Yes | unset | Gotify server URL. |
| `gotify.app_token` | `GOTIFY_APP_TOKEN` | Yes | unset | Gotify application token. |
| `gotify.priority` | `GOTIFY_PRIORITY` | No | `5` | Gotify message priority. |
| `gotify.title_prefix` | `GOTIFY_TITLE_PREFIX` | No | `Forgejo` | Notification title prefix. |
| `forgejo.secret` | `FORGEJO_SECRET` | No | unset | Webhook signature secret. |
| `forgejo.events` | `FORGEJO_EVENTS` | No | `["action_run_failure"]` | Forgejo events to forward. |

Runtime-only environment variables:

| Environment variable | Required | Default | Description |
| --- | --- | --- | --- |
| `APP_CONFIG` | No | `config.toml` | Config file path. |

Docker-specific environment variables:

| Environment variable | Required | Default | Description |
| --- | --- | --- | --- |
| None | No | N/A | The image uses normal app configuration. |

Valid `notification.body_exclude_fields` values are `event`, `repository`, `action`, `prior_status`, `ref`, `commit`, `sender`, `url`, and `delivery`.

Example:

```toml
[notification]
body_exclude_fields = ["ref", "commit", "sender", "prior_status"]
```

When a Forgejo URL can be inferred from the payload, the Gotify message includes `client::notification.click.url` so supported Gotify clients can open that URL when the notification is clicked.

To log full subscribed webhook payloads while testing:

```toml
[logging]
level = "forgejo_actions_failed_webhook_gotify=debug,info"
```

## Forgejo Webhook

Create a Forgejo webhook with:

- Target URL: `http://this-service:3000/webhook`
- HTTP Method: `POST`
- POST Content Type: `application/json`
- Trigger On: all events for a global listener
- Secret: optional, but recommended. Use the same value as `FORGEJO_SECRET`.

Event names are validated against Forgejo `modules/webhook/type.go`. The listener uses `X-Forgejo-Event-Type` when Forgejo sends it, and falls back to `X-Forgejo-Event` for compatibility.

## Forgejo Webhook Configuration

1. As an admin, log into Forgejo and navigate to `/admin`
2. Select Integrations > Webhooks in the left-nav
3. You can use either a System or Default webhook: click the "Add Webhook" button and select "Forgejo"
  a. System webhooks act on **all repositories**, there are security implications here
  b. Default webhooks are only copied to new repositories, and can be reconfigured or removed at the repo level
  c. You can also configure a repository specific webhook by going through the repo's settings
4. Target URL: where ever you have this app deployed. It can be an IP or Domain, and can be HTTP or HTTPS.  Use the `/webhook` path, eg: `https://fwg.myawesomesite.com/webhook`
5. Secret: *optional but recommended* - create a random secret, and ensure it the same in `FORGEJO_SECRET` for this app
6. Trigger on: Select either "push events", or "Custom events..."
  a. If using custom events, check which ever events you want.  For my use-case, I select only Action Run Events / Failure
7. Branch Filter: optional
8. Authentication Header: not implemented, will have no effect
9. Ensure "Active" is checked, and click "Add Webhook"


## Forgejo Server Config / `ALLOWED_HOST_LIST`

By default, Forgejo will not communicate with external hosts unless they are defined in an allow-list.
In v15, this is configured via `gitea.config.webhook.ALLOWED_HOST_LIST`.
There may be other options for configuring this globally, please refer to the official docs for that.
For this app in particular, this is the minimal config required to allow the webhooks to reach your server.

[Forgejo's Configuration Cheat Sheet](https://forgejo.org/docs/latest/admin/config-cheat-sheet/#webhook-webhook) has extensive documentation on other helpful options including proxy configuration, skipping TLS verification, etc.

***By IP Address***:

```yaml
config:
  webhook:
    ALLOWED_HOST_LIST: "external,10.10.13.174"
```

 ***By DNS***:

```yaml
config:
  webhook:
    ALLOWED_HOST_LIST: "external,forgejo-webhooks-gotify.myawesomedomain.com"
```

