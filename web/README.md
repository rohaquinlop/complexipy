# complexipy Web Interface

This is a web interface for complexipy, a tool that calculates cognitive complexity of Python code. The web interface allows users to paste their code and see the cognitive complexity of each line in real-time.

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
├── app.js           # JavaScript for the web application
├── setup.sh         # Script to set up the development environment
├── package.json     # Node.js dependencies and scripts
├── wasm/            # Generated WebAssembly files
    ├── complexipy.js           # JavaScript bindings
    ├── complexipy_wasm.js      # WebAssembly module loader
    ├── complexipy_wasm_bg.wasm # WebAssembly binary
    ├── complexipy.d.ts         # TypeScript definitions
    └── complexipy_wasm.d.ts    # TypeScript definitions
```

## Setup

### Prerequisites

- Rust (with cargo installed)
- Node.js and npm (for the local development server)

### Setting Up the Development Environment

The project includes a setup script that handles all the necessary steps to get the development environment ready:

```bash
# Make sure the script is executable
chmod +x ./setup.sh

# Run the setup script
./setup.sh
```

This will:
1. Check for required dependencies (npm and cargo)
2. Install npm dependencies
3. Build the WebAssembly module
4. Set up the development environment

### Running the Web Application

After setting up the development environment, you can run the web application locally using either:

```bash
# Using npm
npm start

# Or using the serve script
cd .. && ./serve-web-version.sh
```

This will start a local server at `http://localhost:8080`. Open this URL in your browser to use the application.

## Integration with the Main Project

This web interface uses the same cognitive complexity calculation logic as the main complexipy project. The Rust code for calculating cognitive complexity has been adapted to run in the browser via WebAssembly.

## Browser Compatibility

This web application should work in all modern browsers that support WebAssembly:
- Chrome/Edge (latest versions)
- Firefox (latest versions)
- Safari (latest versions)

## License

Same as the main complexipy project