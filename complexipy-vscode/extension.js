// The module 'vscode' contains the VS Code extensibility API
const vscode = require('vscode');
const path = require('path');

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
	console.log('Extension "complexipy-vscode" is now active!');
	console.log('Current extension path:', context.extensionPath);

	const wasmPath = path.join(context.extensionPath, 'wasm', 'complexipy.js');
	const complexityModule = require(wasmPath);

	// The command has been defined in the package.json file
	// Now provide the implementation of the command with  registerCommand
	// The commandId parameter must match the command field in package.json
	let disposable = vscode.commands.registerCommand('complexipy-vscode.cognitiveComplexity', () => {
		const editor = vscode.window.activeTextEditor;
		if (editor) {
			const document = editor.document;
			if (document.languageId === 'python') {
				try {
					const code = document.getText();
					const result = complexityModule.code_complexity(code);
					const functions = result.functions.map(f => `${f.name}: ${f.complexity}`).join(', ');
					vscode.window.showInformationMessage(`Cognitive Complexity: ${functions}`);
				} catch (e) {
					vscode.window.showErrorMessage(`Error analyzing complexity: ${e.message}`);
				}
			} else {
				vscode.window.showInformationMessage('Please open a Python file to analyze.');
			}
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
