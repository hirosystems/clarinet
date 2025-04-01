# Use the official Rust Windows container as base
FROM mcr.microsoft.com/devcontainers/rust:latest

RUN rustup update stable && rustup default stable

# Set working directory
WORKDIR /usr/src/app

# Copy the entire project
COPY . .

# Run cargo test
CMD ["cargo", "test", "--package", "clarity-repl", "--test", "session_with_remote_data", "--", "it_can_fetch_remote", "--exact", "--show-output"]

