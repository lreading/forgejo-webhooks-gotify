# forgejo-actions-failed-webhook-gotify

Listen for Forgejo webhooks and forward subscribed events to Gotify. The default subscription is `action_run_failure`, intended for instance-wide Forgejo webhooks.

## Configuration

The app reads `config.toml` by default. Set `APP_CONFIG=/path/to/config.toml` to use another file. Environment variables override file settings.

Required:

- `GOTIFY_BASE_URL`: Gotify server URL, for example `https://gotify.example.com`
- `GOTIFY_APP_TOKEN`: Gotify application token

Optional:

- `RUST_LOG`, `LOG_LEVEL`, or `APP_LOG_LEVEL`: log level/filter, default `info`
- `BIND_ADDR`: listen address, default `0.0.0.0:3000`
- `WEBHOOK_PATH`: webhook route, default `/webhook`
- `GOTIFY_PRIORITY`: Gotify priority, default `5`
- `GOTIFY_TITLE_PREFIX`: Gotify title prefix, default `Forgejo`
- `FORGEJO_SECRET`: webhook secret used to verify `X-Forgejo-Signature`
- `FORGEJO_EVENTS`: comma-separated Forgejo event names, default `action_run_failure`
- `NOTIFICATION_BODY_EXCLUDE_FIELDS`: comma-separated notification fields to hide

See `config.example.toml` for the file format.

To hide less useful notification body fields:

```toml
[notification]
body_exclude_fields = ["ref", "commit", "sender", "prior_status"]
```

Valid fields are `event`, `repository`, `action`, `prior_status`, `ref`, `commit`, `sender`, `url`, and `delivery`.

When a Forgejo URL can be inferred from the payload, the Gotify message includes
`client::notification.click.url` so supported Gotify clients can open that URL
when the notification is clicked.

To log full subscribed webhook payloads while testing, set:

```toml
[logging]
level = "forgejo_actions_failed_webhook_gotify=debug,info"
```

## Forgejo webhook

Create a Forgejo webhook with:

- Target URL: `http://this-service:3000/webhook`
- HTTP Method: `POST`
- POST Content Type: `application/json`
- Trigger On: all events for a global listener
- Secret: optional, but recommended. Use the same value as `FORGEJO_SECRET`.

Event names are validated against Forgejo `modules/webhook/type.go`. The listener uses `X-Forgejo-Event-Type` when Forgejo sends it, and falls back to `X-Forgejo-Event` for compatibility.

## Docker

TODO: Upload to ghcr

```sh
docker build -t forgejo-webhook-gotify .
docker run --rm -p 3000:3000 \
  -e GOTIFY_BASE_URL=https://gotify.example.com \
  -e GOTIFY_APP_TOKEN=replace-me \
  -e FORGEJO_EVENTS=action_run_failure \
  forgejo-webhook-gotify
```

## TODO:
- Manual testing
- Publish to ghcr
- Document configuring a system webhook on forgejo v15 (or default webhook too?)
- Capture how to filter to a specific message in forgejo
- Capture ALLOWED_HOST_LIST setting
