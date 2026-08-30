#!/bin/bash

# Exit on error
set -e

echo "Building WebAssembly module..."

# Install wasm-pack if it's not already installed
if ! command -v wasm-pack &>/dev/null; then
    echo "Installing wasm-pack..."
    cargo install wasm-pack
fi

# Build the WebAssembly module from the wasm crate
echo "Building with wasm-pack..."
# Note: --out-name is for wasm-pack
(cd crates/complexipy-wasm && wasm-pack build --target web --out-name complexipy_wasm)

# Ensure the wasm directory exists in the web folder
mkdir -p web/wasm
mkdir -p vscode/complexipy/wasm

# Copy the output files to web/wasm directory
echo "Copying generated files..."
cp -r crates/complexipy-wasm/pkg/*.{js,d.ts,wasm} web/wasm/
cp -r crates/complexipy-wasm/pkg/*.{js,d.ts,wasm} vscode/complexipy/wasm/

echo "WebAssembly module built successfully!"
