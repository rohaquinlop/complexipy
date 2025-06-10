#!/bin/bash

# Exit on error
set -e

echo "Starting local development server..."

# Check if http-server is installed
if ! command -v npx &> /dev/null; then
    echo "Error: npx is not installed. Please install Node.js and npm first."
    exit 1
fi

# Start the server
cd web
npx http-server -c-1 -p 8080 .