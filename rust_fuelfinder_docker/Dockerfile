# Stage 1: Compute a recipe
# Use the official lukemathwalker image instead
FROM lukemathwalker/cargo-chef:latest-rust-1.92 AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies
FROM lukemathwalker/cargo-chef:latest-rust-1.92 AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Build the actual application
COPY . .
ENV SQLX_OFFLINE=true
RUN cargo build --release --bin fuelfinder_server

# Stage 4: Lean Runtime
# Using debian-slim makes the final image ~80MB instead of 1GB+
# Stage 4: Runtime
# Use 'trixie' instead of 'bookworm' to get GLIBC 2.38+ support
FROM debian:trixie-slim AS runtime
WORKDIR /app

# Install the essentials
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from the builder stage
# Replace 'fuelfinder_server' with your exact binary name if different
COPY --from=builder /app/target/release/fuelfinder_server /usr/local/bin/app

ENTRYPOINT ["/usr/local/bin/app"]