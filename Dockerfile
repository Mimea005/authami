FROM rust:1.78-slim-bullseye as builder
# FROM rust:1.78-alpine as builder
WORKDIR /usr/src/authami
COPY Cargo.toml Cargo.lock ./
RUN mkdir .cargo
RUN cargo vendor > .cargo/config.toml
COPY src src
RUN cargo build

FROM debian:bullseye-slim
# FROM alpine:3.7
COPY --from=builder /usr/src/authami/target/debug/authami /usr/local/bin/authami
COPY .env .env
COPY public public
ENTRYPOINT ["authami"]
