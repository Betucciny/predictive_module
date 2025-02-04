# Use Arch Linux as the base image
FROM archlinux:latest

# Update package lists and install necessary tools
RUN pacman -Syu --noconfirm \
    base-devel \
    rust \
    cargo \
    openblas \
    lapack \
    openssl \
    git \
    libfbclient \
    && pacman -Scc --noconfirm  # Clean package cache

# Set OpenBLAS to single-threaded mode (avoiding potential threading issues)
ENV OPENBLAS_NUM_THREADS=1
ENV LD_LIBRARY_PATH=/usr/lib

# Create working directory
WORKDIR /app

# Copy the Cargo files separately to optimize caching
COPY Cargo.toml Cargo.lock build.rs ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies first to speed up later builds
RUN cargo build --release

# Remove dummy main.rs and copy actual source code
RUN rm src/main.rs
COPY . .

# Create a new directory for trained models
RUN mkdir -p data

# Build final Rust application
RUN cargo build --release

# Expose the application's port
EXPOSE 3030

# Set the entrypoint to run inside gdb for debugging
ENTRYPOINT ["./target/release/predictive_module"]
