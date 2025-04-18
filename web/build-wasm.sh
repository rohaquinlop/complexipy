#!/bin/bash

# Exit on error
set -e

echo "Building WebAssembly module..."

# Navigate to the wasm/complexipy directory
cd wasm/complexipy

# Install wasm-pack if it's not already installed
if ! command -v wasm-pack &> /dev/null; then
    echo "Installing wasm-pack..."
    cargo install wasm-pack
fi

# Build the WebAssembly module
echo "Building with wasm-pack..."
wasm-pack build --target web

# Create the wasm directory in the web root if it doesn't exist
mkdir -p ../

# Copy the generated files to the web/wasm directory
echo "Copying generated files..."
cp -r pkg/* ../

echo "WebAssembly module built successfully!" 