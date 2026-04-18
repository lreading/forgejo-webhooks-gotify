# Contributing

Keep changes small, focused, and documented when behavior changes.

Before opening a pull request, run the quality gates:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets
cargo build --release
docker build -t forgejo-webhooks-gotify:local .
```

Pull requests must pass CI before merge.

Signed commits are required.
