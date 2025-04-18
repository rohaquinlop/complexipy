// Initialize CodeMirror
let editor;
let complexityModule;

document.addEventListener('DOMContentLoaded', () => {
    // Initialize CodeMirror editor
    const codeEditor = document.getElementById('code-editor');
    editor = CodeMirror.fromTextArea(codeEditor, {
        mode: 'python',
        theme: 'dracula',
        lineNumbers: true,
        lineWrapping: true,
        indentUnit: 4,
        smartIndent: true,
        tabSize: 4,
        indentWithTabs: false,
        autofocus: true,
        extraKeys: { 'Tab': 'insertSoftTab' },
        gutters: ["CodeMirror-linenumbers", "complexity-gutter"]
    });

    // Add sample code
    const sampleCode = `def factorial(n):
    if n <= 1:
        return 1
    else:
        return n * factorial(n - 1)

def calculate_fibonacci(n):
    if n <= 0:
        return 0
    elif n == 1:
        return 1
    else:
        return calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2)

def complex_function(data, threshold=10):
    result = []
    
    for item in data:
        if item > threshold:
            if item % 2 == 0:
                result.append(item * 2)
            else:
                result.append(item * 3)
        elif item < 0:
            continue
        else:
            if item % 3 == 0:
                result.append(item // 3)
            elif item % 2 == 0:
                result.append(item // 2)
            else:
                result.append(item)
    
    return result
`;
    editor.setValue(sampleCode);

    // Analyze button click handler
    const analyzeBtn = document.getElementById('analyze-btn');
    analyzeBtn.addEventListener('click', analyzeCode);

    // Initialize the WASM module
    initWasmModule();
});

async function initWasmModule() {
    try {
        // Show a loading indicator or message here if needed
        const analyzeBtn = document.getElementById('analyze-btn');
        analyzeBtn.textContent = 'Loading...';
        analyzeBtn.disabled = true;

        // Initialize the WebAssembly module
        complexityModule = await import('./wasm/complexipy_wasm.js');
        await complexityModule.default();

        // Enable the analyze button once the module is loaded
        analyzeBtn.textContent = 'Analyze';
        analyzeBtn.disabled = false;

        // Analyze the sample code
        analyzeCode();
    } catch (error) {
        console.error('Failed to load WebAssembly module:', error);
        document.getElementById('analyze-btn').textContent = 'Error loading analyzer';
    }
}

function analyzeCode() {
    // Get the code from the editor
    const code = editor.getValue();

    if (!code.trim() || !complexityModule) {
        return;
    }

    try {
        // Call the WebAssembly function to analyze the code
        const result = complexityModule.code_complexity(code);
        updateUI(result);
    } catch (error) {
        console.error('Analysis error:', error);
        showError(error.message || 'An error occurred during analysis');
    }
}

function updateUI(result) {
    // Update summary
    document.getElementById('total-complexity').textContent = result.complexity;
    document.getElementById('function-count').textContent = result.functions.length;

    // Update function list
    const functionsListElement = document.getElementById('functions-list');
    functionsListElement.innerHTML = '';

    result.functions.forEach(func => {
        const functionItem = document.createElement('div');
        functionItem.className = 'function-item';

        // Determine complexity class
        let complexityClass = 'complexity-low';
        if (func.complexity > 15) {
            complexityClass = 'complexity-high';
        } else if (func.complexity > 5) {
            complexityClass = 'complexity-medium';
        }

        // Create more detailed function display
        functionItem.innerHTML = `
            <div class="function-item-header">
                <span class="function-name">${func.name}</span>
                <span class="function-complexity ${complexityClass}">${func.complexity}</span>
            </div>
            <div class="function-details">
                <div class="function-location">Line: ${func.line_start}</div>
                <div class="function-cognitive-info">
                    <span class="function-cognitive-label">Cognitive Complexity:</span>
                    <span class="function-cognitive-value ${complexityClass}">${func.complexity}</span>
                </div>
            </div>
        `;

        // Add click event to jump to function in editor
        functionItem.addEventListener('click', () => {
            editor.setCursor(func.line_start - 1);
            editor.focus();
        });

        functionsListElement.appendChild(functionItem);
    });

    // Add complexity indicators to each line
    addComplexityIndicators(result);
}

function addComplexityIndicators(result) {
    // Clear all gutters first
    for (let i = 0; i < editor.lineCount(); i++) {
        editor.setGutterMarker(i, "complexity-gutter", null);
    }

    // Create a map of line numbers to complexity values
    const lineComplexityMap = new Map();

    // Also track function definition lines and their total complexity
    const functionComplexityMap = new Map();

    // Process functions and their statements
    result.functions.forEach(func => {
        // Add function total complexity to the function start line
        functionComplexityMap.set(func.line_start, func.complexity);

        // If we have line-by-line complexity data, add it to our map
        if (func.line_complexities) {
            func.line_complexities.forEach(lineComp => {
                lineComplexityMap.set(lineComp.line, lineComp.complexity);
            });
        }
    });

    // Add markers for function complexity (add these first)
    functionComplexityMap.forEach((complexity, lineNum) => {
        // Create the marker element
        const marker = document.createElement("div");
        marker.className = "complexity-marker function-complexity-marker";

        // Set color based on complexity
        let bgColor = "#50fa7b"; // low - green
        if (complexity > 15) {
            bgColor = "#ff5555"; // high - red
        } else if (complexity > 5) {
            bgColor = "#ffb86c"; // medium - orange
        }

        marker.innerHTML = `<span style="background-color:${bgColor};">${complexity}</span>`;

        // Set the gutter marker
        editor.setGutterMarker(lineNum - 1, "complexity-gutter", marker);
    });

    // Add markers for each line with complexity (but don't overwrite function markers)
    lineComplexityMap.forEach((complexity, lineNum) => {
        // Skip if this is a function definition line (we already added a marker)
        if (functionComplexityMap.has(lineNum)) {
            return;
        }

        if (complexity > 0) {
            // Create the marker element
            const marker = document.createElement("div");
            marker.className = "complexity-marker";

            // Set color based on complexity
            let bgColor = "#50fa7b"; // low - green
            if (complexity > 5) {
                bgColor = "#ff5555"; // high - red
            } else if (complexity > 2) {
                bgColor = "#ffb86c"; // medium - orange
            }

            marker.innerHTML = `<span style="background-color:${bgColor};">${complexity}</span>`;

            // Set the gutter marker
            editor.setGutterMarker(lineNum - 1, "complexity-gutter", marker);
        }
    });
}

function showError(message) {
    // Display error to the user
    const summaryElement = document.getElementById('summary-content');
    summaryElement.innerHTML = `<div class="error-message">${message}</div>`;

    // Clear function list
    document.getElementById('functions-list').innerHTML = '';

    // Clear indicators
    document.getElementById('complexity-indicators').innerHTML = '';
} 