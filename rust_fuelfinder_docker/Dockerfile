# Stage 1: Compute a recipe
# Use the official lukemathwalker image instead
FROM lukemathwalker/cargo-chef:latest-rust-1.80 AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Build dependencies
FROM lukemathwalker/cargo-chef:latest-rust-1.80 AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 3: Build the actual application
COPY . .
RUN cargo build --release --bin fuelfinder_server

# Stage 4: Lean Runtime
# Using debian-slim makes the final image ~80MB instead of 1GB+
FROM debian:bookworm-slim AS runtime
WORKDIR /app
# Install OpenSSL as it's a common dependency for web servers
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
# Copy only the compiled binary from the builder
COPY --from=builder /app/target/release/fuelfinder_server /usr/local/bin/app
# Set the execution command
ENTRYPOINT ["/usr/local/bin/app"]