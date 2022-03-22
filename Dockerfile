# syntax=docker/dockerfile:1

# code from https://hub.docker.com/_/rust
FROM rust:1.59 as builder
WORKDIR /usr/src/musicthing
COPY . .
RUN cargo install --path .

# create the database container image
FROM postgres:14
ENV POSTGRES_USER postgres
ENV POSTGRES_PASSWORD password
ENV POSTGRES_DB musicthing-metadb
COPY musicthing_metadb_init.sql /docker-entrypoint-initdb.d/

# move the rust binary inside
RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
EXPOSE 8000

COPY --from=builder /usr/local/cargo/bin/musicthing /usr/local/bin/musicthing
#CMD ["musicthing"]
