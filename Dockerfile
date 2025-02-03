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

# Create symbolic links for libfbclient
RUN ln -s /usr/lib/x86_64-linux-gnu/libfbclient.so.2 /usr/lib/libfbclient.so.2 \
    && ln -s /usr/lib/libfbclient.so.2 /usr/lib/libfbclient.so \
    && ln -s /usr/lib/libfbclient.so /usr/lib/libgds.so.0 \
    && ln -s /usr/lib/libfbclient.so /usr/lib/libgds.so

# Set the library path
ENV LD_LIBRARY_PATH=/usr/lib/x86_64-linux-gnu:/usr/lib

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

#Create a new directory for the trained models
RUN mkdir data

# Build the application
RUN cargo build --release

EXPOSE 3030

# Set the entrypoint to the built binary
ENTRYPOINT ["./target/release/predictive_module"]
