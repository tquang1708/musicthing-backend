# syntax=docker/dockerfile:1
# adapted from https://blog.logrocket.com/packaging-a-rust-web-service-using-docker/
FROM rust:1.59 as builder

# create an empty rust project, build it then remove source to make dependencies
RUN cargo new --bin musicthing
WORKDIR ./musicthing
COPY Cargo.toml Cargo.toml
RUN cargo build --release
RUN rm src/*

## copy project's source
COPY . ./

# rebuild
RUN rm ./target/release/deps/musicthing*
RUN cargo build --release

# create a container image
FROM ubuntu:focal
ARG APP=/usr/src/app
RUN apt-get update
EXPOSE 8000

COPY --from=builder /musicthing/target/release/musicthing ${APP}/musicthing
WORKDIR ${APP}
#CMD ["./musicthing"]
