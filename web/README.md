# ComplexiPy Web Interface

This is a web interface for ComplexiPy, a tool that calculates cognitive complexity of Python code. The web interface allows users to paste their code and see the cognitive complexity of each line in real-time.

## Features

- Syntax highlighted code editor
- Line-by-line cognitive complexity indicators
- Function-level complexity analysis
- Summary of total complexity

## Project Structure

```
web/
├── index.html        # Main HTML file
├── styles.css        # CSS styles
├── app.js            # JavaScript for the web application
├── build-wasm.sh     # Script to build the WebAssembly module
├── serve.sh          # Script to start the local server
├── wasm/             # WebAssembly module source and output
    └── complexipy/   # Rust code for the WebAssembly module
        ├── Cargo.toml
        └── src/
            └── lib.rs
```

## Setup

### Prerequisites

- Rust (with cargo installed)
- Node.js and npm (for the local development server)

### Building the WebAssembly Module

The WebAssembly module contains the Rust code that calculates the cognitive complexity of Python code. To build it:

```bash
# Make sure the script is executable
chmod +x ./build-wasm.sh

# Run the build script
./build-wasm.sh
```

This will:
1. Install wasm-pack if it's not already installed
2. Build the WebAssembly module
3. Copy the generated files to the web/wasm directory

### Running the Web Application

After building the WebAssembly module, you can run the web application locally:

```bash
# Make sure the script is executable
chmod +x ./serve.sh

# Run the server
./serve.sh
```

This will start a local server at `http://localhost:8080`. Open this URL in your browser to use the application.

## Integration with the Main Project

This web interface uses the same cognitive complexity calculation logic as the main ComplexiPy project. The Rust code for calculating cognitive complexity has been adapted to run in the browser via WebAssembly.

## Browser Compatibility

This web application should work in all modern browsers that support WebAssembly:
- Chrome/Edge (latest versions)
- Firefox (latest versions)
- Safari (latest versions)

## License

Same as the main ComplexiPy project 