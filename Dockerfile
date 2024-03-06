# Stage 1: Build environment
FROM rust:1.76-alpine3.19 as builder

RUN apk add --no-cache musl-dev

WORKDIR /usr/src/brutus

# Copy the source code of your project
COPY . .

# Build your project
RUN cargo install --path .

# Stage 2: Runtime environment
FROM alpine:3.19.1

# Create a new user "brutus" with no login capabilities for running the application
RUN adduser -D -h /opt/brutus -s /sbin/nologin brutus

WORKDIR /opt/brutus

# Install dependencies for the runtime environment
RUN apk --no-cache add ca-certificates

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/local/cargo/bin/brutus .
COPY --from=builder /usr/src/brutus/docs ./docs
RUN chown -R brutus:brutus /opt/brutus

# Expose the port the application listens on
EXPOSE 8080

# Switch to the brutus user
USER brutus

# Run the binary
ENTRYPOINT ["/opt/brutus/brutus"]

