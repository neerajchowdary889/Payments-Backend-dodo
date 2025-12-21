# Builder stage
FROM rust:alpine as builder

WORKDIR /usr/src/app

# Install build dependencies for Alpine/Musl
# pkgconfig and openssl-dev are needed for compiling dependencies like reqwest/sqlx with native TLS if enabled
# musl-dev is required for standard C headers
RUN apk add --no-cache pkgconfig musl-dev openssl-dev

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source to build dependencies
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Copy actual source code
COPY . .

# Touch main.rs to ensure rebuild
RUN touch src/main.rs

# Build the application
RUN cargo build --release

# Runtime stage
FROM alpine:latest

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /usr/src/app/target/release/payments-backend-dodo /usr/local/bin/app

# Expose the application port
EXPOSE 3000

# Run the application
CMD ["app"]
