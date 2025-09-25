# Building the AMP Rust Kernel

The Agentic Mesh Protocol (AMP) kernel is written in Rust and needs to be compiled separately from the TypeScript components.

## Prerequisites

1. Rust toolchain (latest stable version)
2. Cargo package manager (comes with Rust)

## Installing Rust

To install Rust, run the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the prompts to complete the installation. After installation, restart your shell or run:

```bash
source $HOME/.cargo/env
```

## Building the Kernel

Once Rust is installed, navigate to the kernel directory and build:

```bash
cd kernel
cargo build --release
```

This will compile the kernel and create the binary in `target/release/`.

## Running the Kernel

After building, you can run the kernel:

```bash
# Run the kernel API server
cargo run --bin kernel-api

# Or run the CLI tool
cargo run --bin ampctl -- help
```

## Cross-compilation (Optional)

If you need to build for a different target platform:

```bash
# Add the target
rustup target add <target-triple>

# Build for the target
cargo build --target <target-triple> --release
```

## Docker Alternative

If you prefer not to install Rust locally, you can use the provided Docker configuration:

```bash
# Build the kernel using Docker
docker build -f Dockerfile.kernel -t amp-kernel .

# Run the kernel in a container
docker run -p 7777:7777 amp-kernel
```