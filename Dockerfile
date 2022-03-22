# dockerfile for the backend image in docker-compose

# code from https://hub.docker.com/_/rust
FROM rust:1.59 as builder
WORKDIR /usr/src/musicthing
COPY . .
RUN cargo install --path .

# create the database container image
FROM ubuntu:devel
RUN apt-get update
EXPOSE 8000

COPY --from=builder /usr/local/cargo/bin/musicthing /usr/local/bin/musicthing
# COPY ./target/release/musicthing /usr/local/bin/musicthing
#CMD ["musicthing"]
