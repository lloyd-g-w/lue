FROM rust:1-bookworm AS builder

RUN rustup target add wasm32-unknown-unknown \
    && cargo install dioxus-cli --version 0.6.3 --locked

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p server
RUN dx build --package web --release

FROM debian:bookworm-slim AS server-runtime

WORKDIR /app
RUN mkdir -p /data
COPY --from=builder /app/target/release/server /usr/local/bin/server

ENV SERVER_ADDR=0.0.0.0:3000
ENV DATA_PATH=/data/store.json
EXPOSE 3000

CMD ["server"]

FROM nginx:1.27-alpine AS web-runtime

COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/target/dx/web/release/web/public /usr/share/nginx/html

EXPOSE 80
