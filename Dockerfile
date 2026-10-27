# RustPress as a small container image.
#
# The binary is linked statically against musl, so the same build also runs outside containers
# (for example as a CGI program on shared hosting).
#
#   docker build -t rustpress .
#   docker run -p 3000:3000 -v /pfad/zu/wordpress:/var/www/html \
#     -e RUSTPRESS_DATABASE_URL=mysql://user:pass@db:3306/wordpress rustpress

# The Alpine Rust image already brings gcc and musl-dev (needed by ring).
FROM rust:1-alpine AS build
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p rustpress-server \
 && cp target/release/rustpress-server /rustpress

FROM alpine:3.22
# Alpine already ships the CA bundle (for TLS connections to the database).
RUN adduser -D -H -u 10001 rustpress
COPY --from=build /rustpress /usr/local/bin/rustpress
USER rustpress
ENV RUSTPRESS_LISTEN=0.0.0.0:3000 \
    RUSTPRESS_WP_ROOT=/var/www/html \
    RUST_LOG=info
EXPOSE 3000
# The server shuts down gracefully on Ctrl+C (SIGINT); `docker stop` sends SIGTERM by default.
STOPSIGNAL SIGINT
ENTRYPOINT ["/usr/local/bin/rustpress"]
