# ── Stage 1: Build the Rust backend ──
FROM rust:1.76 AS builder

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /usr/src/app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main to cache dependencies
RUN mkdir src && echo "fn main(){}" > src/main.rs
ENV SQLX_OFFLINE=true
RUN cargo build --target x86_64-unknown-linux-musl --release
RUN rm src/main.rs

# Copy real source code
COPY ./src ./src
COPY ./.sqlx ./.sqlx

# Build the actual application
RUN rm -f target/x86_64-unknown-linux-musl/release/deps/multi_lang_compiler*
RUN cargo build --target x86_64-unknown-linux-musl --release


# ── Stage 2: Production runtime with all language compilers ──
FROM debian:bookworm-slim

# Prevent interactive prompts during install
ENV DEBIAN_FRONTEND=noninteractive

# Install ALL language runtimes and compilers
RUN apt-get update && apt-get install -y --no-install-recommends \
    # C / C++
    gcc \
    g++ \
    libc-dev \
    # Python
    python3 \
    python3-dev \
    # Node.js (via nodesource for latest LTS)
    nodejs \
    npm \
    # Go
    golang \
    # Java (OpenJDK)
    default-jdk-headless \
    # Rust compiler (for user Rust code)
    rustc \
    # Utilities
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# ── Pre-warm compilers (first invocation is always slow) ──
# This runs each compiler once during build so the first user request is fast
RUN echo 'int main(){return 0;}' > /tmp/warmup.c && \
    gcc /tmp/warmup.c -o /tmp/warmup_c 2>/dev/null && \
    rm -f /tmp/warmup.c /tmp/warmup_c || true

RUN echo '#include <iostream>\nint main(){return 0;}' > /tmp/warmup.cpp && \
    g++ /tmp/warmup.cpp -o /tmp/warmup_cpp 2>/dev/null && \
    rm -f /tmp/warmup.cpp /tmp/warmup_cpp || true

RUN echo 'fn main(){}' > /tmp/warmup.rs && \
    rustc /tmp/warmup.rs -o /tmp/warmup_rs 2>/dev/null && \
    rm -f /tmp/warmup.rs /tmp/warmup_rs || true

RUN echo 'public class Warmup{public static void main(String[] a){}}' > /tmp/Warmup.java && \
    javac /tmp/Warmup.java 2>/dev/null && \
    rm -f /tmp/Warmup.java /tmp/Warmup.class || true

RUN python3 -c "import sys; print(sys.version)" 2>/dev/null || true

RUN echo 'package main\nfunc main(){}' > /tmp/warmup.go && \
    go run /tmp/warmup.go 2>/dev/null && \
    rm -f /tmp/warmup.go || true

RUN node -e "console.log('ok')" 2>/dev/null || true

# Create tmpfs mount point for fast temp files
RUN mkdir -p /tmp/codeconnect

# Copy the compiled binary
COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/multi-lang-compiler /usr/local/bin/

# Environment
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["multi-lang-compiler"]