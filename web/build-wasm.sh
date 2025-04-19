#!/bin/bash

# Exit on error
set -e

echo "Building WebAssembly module..."

# Install wasm-pack if it's not already installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    cargo install wasm-pack
fi

# Build the WebAssembly module with the wasm feature
echo "Building with wasm-pack..."
# Note: --out-name is for wasm-pack, while --features and --no-default-features are for cargo
wasm-pack build --target web --out-name complexipy_wasm -- --features wasm --no-default-features

# Ensure the wasm directory exists in the web folder
mkdir -p web/wasm

# Copy the output files to web/wasm directory
echo "Copying generated files..."
cp -r pkg/* web/wasm/

echo "WebAssembly module built successfully!" 