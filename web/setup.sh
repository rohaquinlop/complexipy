#!/bin/bash

# Exit on error
set -e

echo "Setting up complexipy Web Interface..."

# Check if npm is installed
if ! command -v npm &> /dev/null; then
    echo "Error: npm is not installed. Please install Node.js and npm first."
    exit 1
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    echo "Error: cargo is not installed. Please install Rust and Cargo first."
    echo "Visit https://rustup.rs/ for installation instructions."
    exit 1
fi

# Install npm dependencies
echo "Installing npm dependencies..."
npm install

# Build the WebAssembly module
echo "Building WebAssembly module..."
chmod +x ./build-wasm.sh
./build-wasm.sh

echo "Setup complete!"
echo "To start the development server, run: npm start"
echo "Or: ./serve.sh" 