
FROM rust:1.76 AS builder

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN mkdir src && echo "fn main(){}" > src/main.rs
# Build dependencies first to cache them
RUN cargo build --target x86_64-unknown-linux-musl --release
RUN rm src/main.rs

COPY ./src ./src

# Build the actual application
RUN rm -f target/x86_64-unknown-linux-musl/release/deps/multi_lang_compiler*
RUN cargo build --target x86_64-unknown-linux-musl --release

# Use a fuller image for runtime to support other languages
FROM debian:bookworm-slim

# Install necessary runtimes and compilers
RUN apt-get update && apt-get install -y \
    python3 \
    gcc \
    g++ \
    nodejs \
    golang \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/multi-lang-compiler /usr/local/bin/

EXPOSE 8080

CMD ["multi-lang-compiler"]