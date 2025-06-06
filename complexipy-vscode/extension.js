// The module 'vscode' contains the VS Code extensibility API
const vscode = require('vscode');
const path = require('path');

const lowComplexityDecorationType = vscode.window.createTextEditorDecorationType({
	isWholeLine: true,
	dark: {
		after: {
			margin: '0 0 0 1em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: '#50fa7b',
			color: '#282a36',
		}
	},
	light: {
		after: {
			margin: '0 0 0 1em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: '#28a745',
			color: '#ffffff',
		}
	}
});

const mediumComplexityDecorationType = vscode.window.createTextEditorDecorationType({
	isWholeLine: true,
	dark: {
		after: {
			margin: '0 0 0 1em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: '#ffb86c',
			color: '#282a36',
		}
	},
	light: {
		after: {
			margin: '0 0 0 1em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: '#ffc107',
			color: '#212529',
		}
	}
});

const highComplexityDecorationType = vscode.window.createTextEditorDecorationType({
	isWholeLine: true,
	dark: {
		after: {
			margin: '0 0 0 1em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: '#ff5555',
			color: '#FFFFFF',
		}
	},
	light: {
		after: {
			margin: '0 0 0 1em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: '#dc3545',
			color: '#ffffff',
		}
	}
});

const allDecorationTypes = [lowComplexityDecorationType, mediumComplexityDecorationType, highComplexityDecorationType];

function analyzeAndDecorate(editor, complexityModule) {
	if (!editor) return;

	const { document } = editor;
	if (document.languageId !== 'python') {
		return;
	}

	try {
		const code = document.getText();
		const result = complexityModule.code_complexity(code);

		const lowComplexityDecorations = [];
		const mediumComplexityDecorations = [];
		const highComplexityDecorations = [];

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
			if (line >= 0 && line < document.lineCount) {
				const position = new vscode.Position(line, document.lineAt(line).range.end.character);
				const range = new vscode.Range(position, position);
				const decoration = { range, renderOptions: { after: { contentText: ` ${complexity}` } } };

				if (complexity > 15) {
					highComplexityDecorations.push(decoration);
				} else if (complexity > 5) {
					mediumComplexityDecorations.push(decoration);
				} else {
					lowComplexityDecorations.push(decoration);
				}
			}
		});

		lineComplexityMap.forEach((complexity, lineNum) => {
			if (functionComplexityMap.has(lineNum)) {
				return;
			}
			const line = lineNum - 1;
			if (line >= 0 && line < document.lineCount) {
				const position = new vscode.Position(line, document.lineAt(line).range.end.character);
				const range = new vscode.Range(position, position);
				const decoration = { range, renderOptions: { after: { contentText: ` ${complexity}` } } };

				if (complexity > 5) {
					highComplexityDecorations.push(decoration);
				} else if (complexity > 2) {
					mediumComplexityDecorations.push(decoration);
				} else {
					lowComplexityDecorations.push(decoration);
				}
			}
		});

		editor.setDecorations(lowComplexityDecorationType, lowComplexityDecorations);
		editor.setDecorations(mediumComplexityDecorationType, mediumComplexityDecorations);
		editor.setDecorations(highComplexityDecorationType, highComplexityDecorations);
	} catch (e) {
		console.error(`Error analyzing complexity: ${e.message}`);
		allDecorationTypes.forEach(decorationType => {
			editor.setDecorations(decorationType, []);
		});
	}
}

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
	console.log('Extension "complexipy-vscode" is now active!');
	console.log('Current extension path:', context.extensionPath);

	const wasmPath = path.join(context.extensionPath, 'wasm', 'complexipy.js');
	const complexityModule = require(wasmPath);

	let activeEditor = vscode.window.activeTextEditor;

	let timeout;
	function triggerAnalysis() {
		if (timeout) {
			clearTimeout(timeout);
		}
		timeout = setTimeout(() => {
			analyzeAndDecorate(activeEditor, complexityModule);
		}, 100);
	}

	if (activeEditor) {
		triggerAnalysis();
	}

	vscode.window.onDidChangeActiveTextEditor(editor => {
		activeEditor = editor;
		if (editor) {
			triggerAnalysis();
		}
	}, null, context.subscriptions);

	vscode.workspace.onDidSaveTextDocument(document => {
		if (activeEditor && document === activeEditor.document) {
			triggerAnalysis();
		}
	}, null, context.subscriptions);

	vscode.workspace.onDidChangeTextDocument(event => {
		if (activeEditor && event.document === activeEditor.document) {
			allDecorationTypes.forEach(decorationType => {
				activeEditor.setDecorations(decorationType, []);
			});
		}
	}, null, context.subscriptions);

	// The command has been defined in the package.json file
	// Now provide the implementation of the command with  registerCommand
	// The commandId parameter must match the command field in package.json
	let disposable = vscode.commands.registerCommand('complexipy-vscode.cognitiveComplexity', () => {
		if (activeEditor && activeEditor.document.languageId === 'python') {
			analyzeAndDecorate(activeEditor, complexityModule);
		} else if (activeEditor) {
			vscode.window.showInformationMessage('Please open a Python file to analyze.');
		} else {
			vscode.window.showInformationMessage('No active editor found.');
		}
	});

	context.subscriptions.push(disposable);
	console.log('Command registered successfully');
}

// This method is called when your extension is deactivated
function deactivate() {
	console.log('Extension "complexipy-vscode" is now deactivated');
}

module.exports = {
	activate,
	deactivate
}
