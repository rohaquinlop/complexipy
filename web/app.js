// Initialize CodeMirror
let editor;
let complexityModule;
let analysisTimeout;
let activeWidgets = [];

document.addEventListener('DOMContentLoaded', () => {
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
        viewportMargin: Infinity,
        scrollbarStyle: 'native',
        extraKeys: { 'Tab': 'insertSoftTab' },
        gutters: ["CodeMirror-linenumbers"],
        styleActiveLine: {
            nonEmpty: true,
            gutter: false
        }
    });

    const refreshLayout = () => {
        setTimeout(() => {
            editor.refresh();
        }, 100);
    };

    window.addEventListener('resize', refreshLayout);
    refreshLayout();

    editor.on('change', () => {
        if (analysisTimeout) {
            clearTimeout(analysisTimeout);
        }
        analysisTimeout = setTimeout(() => {
            analyzeCode();
        }, 300);
    });

    initWasmModule();
});

async function initWasmModule() {
    const statusElement = document.getElementById('analysis-status');
    try {
        statusElement.textContent = 'Loading Analyzer...';
        complexityModule = await import('./wasm/complexipy_wasm.js');
        await complexityModule.default();
        statusElement.textContent = 'Ready';
        analyzeCode();
    } catch (error) {
        console.error('Failed to load WebAssembly module:', error);
        statusElement.textContent = 'Error loading analyzer';
    }
}

function analyzeCode() {
    if (!editor || !complexityModule) {
        return;
    }

    const code = editor.getValue();
    const statusElement = document.getElementById('analysis-status');

    // Clear previous widgets
    activeWidgets.forEach(widget => widget.clear());
    activeWidgets = [];

    if (!code.trim()) {
        statusElement.textContent = 'Ready';
        return;
    }

    try {
        statusElement.textContent = 'Analyzing...';
        const result = complexityModule.code_complexity(code);
        addComplexityDecorations(result);
        statusElement.textContent = 'Ready';
    } catch (error) {
        console.error('Analysis error:', error);
        statusElement.textContent = 'Analysis Failed';
    }
}

function addComplexityDecorations(result) {
    if (!result || !editor) return;

    activeWidgets.forEach(widget => widget.clear());
    activeWidgets = [];

    const lineComplexityMap = new Map();
    const functionComplexityMap = new Map();

    result.functions.forEach(func => {
        functionComplexityMap.set(func.line_start, func.complexity);
        if (func.line_complexities && func.line_complexities.length > 0) {
            const lineAggregates = new Map();
            func.line_complexities.forEach(lineComp => {
                if (lineComp.complexity > 0) {
                    const line = lineComp.line;
                    const currentValue = lineAggregates.get(line) || 0;
                    lineAggregates.set(line, currentValue + lineComp.complexity);
                }
            });
            lineAggregates.forEach((complexity, line) => {
                const existingComplexity = lineComplexityMap.get(line) || 0;
                lineComplexityMap.set(line, existingComplexity + complexity);
            });
        }
    });

    functionComplexityMap.forEach((complexity, lineNum) => {
        const line = lineNum - 1;
        if (line >= 0 && line < editor.lineCount() && complexity > 0) {
            const widget = document.createElement('span');
            widget.className = 'complexity-widget';
            widget.textContent = `ƒ ${complexity}`;

            if (complexity >= 15) {
                widget.classList.add('high-complexity');
            } else {
                widget.classList.add('low-complexity');
            }

            const lineContent = editor.getLine(line);
            if (lineContent.trim().length > 0) {
                const position = { line: line, ch: lineContent.length };
                const bookmark = editor.setBookmark(position, { widget: widget, insertLeft: true });
                activeWidgets.push(bookmark);
            }
        }
    });

    lineComplexityMap.forEach((complexity, lineNum) => {
        if (functionComplexityMap.has(lineNum)) {
            return;
        }
        const line = lineNum - 1;
        if (line >= 0 && line < editor.lineCount()) {
            const widget = document.createElement('span');
            widget.className = 'complexity-widget';
            widget.textContent = `+${complexity}`;

            if (complexity > 5) {
                widget.classList.add('high-complexity');
            } else {
                widget.classList.add('low-complexity');
            }

            const lineContent = editor.getLine(line);
            if (lineContent.trim().length > 0) {
                const position = { line: line, ch: lineContent.length };
                const bookmark = editor.setBookmark(position, { widget: widget, insertLeft: true });
                activeWidgets.push(bookmark);
            }
        }
    });
}