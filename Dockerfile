# Build stage
FROM rust:trixie AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install trunk and add wasm target
RUN cargo install trunk && \
    rustup target add wasm32-unknown-unknown

# Set the working directory
WORKDIR /usr/src/renoma

# Copy the entire workspace
COPY . .

# Build the distribution
RUN cargo xtask dist

# Runtime stage
FROM debian:trixie

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the distribution directory from the builder stage
COPY --from=builder /usr/src/renoma/Renoma /app

EXPOSE 8080

CMD ["./renoma-launcher"]
