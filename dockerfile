# Use Ubuntu 22.04 as the base image
FROM ubuntu:22.04

# Install Rust, pkg-config, and OpenSSL development libraries
RUN apt-get update && \
    apt-get install -y curl build-essential pkg-config libssl-dev && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Set the environment variable for Rust
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy your Rust project into the Docker image
COPY . /usr/src/myapp

# Set the working directory
WORKDIR /usr/src/myapp

# Compile the Rust project
RUN cargo build --release

# Clear docker
# docker system prune -a
# docker volume prune

# docker buildx build --platform linux/amd64 -t bellman_ford_pegasus .
# docker create --name temp-container bellman_ford_pegasus
# docker cp temp-container:/usr/src/myapp/target/release/bellman_ford_pegasus .
# docker rm temp-container

# upload the bellman_ford_pegasus file to an aws S3 bucket
# set EC2 IAM to s3:GetObject

# sudo apt install awscli
# aws s3 cp s3://cwizards_playground/bellman_ford_pegasus /home/ubuntu/bellman_ford_pegasus
# chmod +x bellman_ford_pegasus
