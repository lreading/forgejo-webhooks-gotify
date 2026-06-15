FROM alpine:3.24.0 AS builder

WORKDIR /app
ENV PATH="/root/.cargo/bin:${PATH}"

RUN apk add --no-cache ca-certificates gcc musl-dev rustup
COPY rust-toolchain.toml ./
RUN rustup-init -y --no-modify-path --profile minimal --default-toolchain none \
    && rustup show active-toolchain

COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM alpine:3.24.0

ARG IMAGE_CREATED=1970-01-01T00:00:00Z
ARG IMAGE_REVISION=local-test-build
ARG IMAGE_SOURCE=https://github.com/lreading/forgejo-webhooks-gotify
ARG IMAGE_VERSION=v0.0.0-testing

LABEL org.opencontainers.image.title="forgejo-webhooks-gotify" \
    org.opencontainers.image.description="Forward selected Forgejo webhooks to Gotify." \
    org.opencontainers.image.authors="Leo Reading <leo.reading@owasp.org>" \
    org.opencontainers.image.url="${IMAGE_SOURCE}" \
    org.opencontainers.image.source="${IMAGE_SOURCE}" \
    org.opencontainers.image.documentation="${IMAGE_SOURCE}#readme" \
    org.opencontainers.image.version="${IMAGE_VERSION}" \
    org.opencontainers.image.revision="${IMAGE_REVISION}" \
    org.opencontainers.image.created="${IMAGE_CREATED}" \
    org.opencontainers.image.licenses="Apache-2.0"

RUN apk add --no-cache ca-certificates \
    && addgroup -S forgejo-webhook \
    && adduser -S -D -H -G forgejo-webhook -s /bin/sh forgejo-webhook

COPY --from=builder /app/target/release/forgejo-webhooks-gotify /usr/local/bin/forgejo-webhook-gotify

USER forgejo-webhook:forgejo-webhook

ENTRYPOINT ["/usr/local/bin/forgejo-webhook-gotify"]
