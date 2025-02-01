FROM rust:latest

# Install required packages
RUN apt-get update && apt-get install -y \
    libfbclient2 \
    liblapack-dev \
    libblas-dev \
    libopenblas-dev \
    gfortran \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Create a new directory for the application
WORKDIR /app

# Copy the Cargo.toml and Cargo.lock files
COPY Cargo.toml Cargo.lock build.rs ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build the dependencies
RUN cargo build --release

# Remove the dummy main.rs
RUN rm src/main.rs

# Copy the source code
COPY . .

# Build the application
RUN cargo build --release

# Set the entrypoint to the built binary
ENTRYPOINT ["./target/release/your_project_name"]
