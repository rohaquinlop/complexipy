const vscode = require('vscode');
const path = require('path');
const fs = require('fs').promises;

let statusBarItem;

const lowComplexityDecorationType = vscode.window.createTextEditorDecorationType({
	dark: {
		after: {
			margin: '0 0.5em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: 'rgba(61, 139, 64, 0.8)',
			color: '#FFFFFF',
		}
	},
	light: {
		after: {
			margin: '0 0.5em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: 'rgba(212, 237, 218, 0.8)',
			color: '#155724',
		}
	}
});

const highComplexityDecorationType = vscode.window.createTextEditorDecorationType({
	dark: {
		after: {
			margin: '0 0.5em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: 'rgba(166, 51, 51, 0.8)',
			color: '#FFFFFF',
		}
	},
	light: {
		after: {
			margin: '0 0.5em',
			textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;',
			backgroundColor: 'rgba(248, 215, 218, 0.8)',
			color: '#721C24',
		}
	}
});

const allDecorationTypes = [lowComplexityDecorationType, highComplexityDecorationType];

function updateStatusBar(fileStats) {
	if (!statusBarItem) return;

	if (!fileStats) {
		statusBarItem.hide();
		return;
	}

	const { totalFunctions, highComplexityFunctions, averageComplexity, maxComplexity, fileName } = fileStats;

	if (totalFunctions === 0) {
		statusBarItem.text = `$(symbol-function) No functions`;
		statusBarItem.tooltip = `No functions found in ${fileName}`;
		statusBarItem.backgroundColor = undefined;
		statusBarItem.show();
		return;
	}

	let icon = '$(symbol-function)';
	let backgroundColor = undefined;
	let status = 'Good';

	if (highComplexityFunctions > 0) {
		icon = '$(warning)';
		backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
		status = 'Needs Review';
	}

	if (highComplexityFunctions > totalFunctions * 0.5) {
		icon = '$(error)';
		backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
		status = 'Urgent';
	}

	statusBarItem.text = `${icon} ${totalFunctions} functions (${highComplexityFunctions} high)`;
	statusBarItem.backgroundColor = backgroundColor;

	const complexityGrade = maxComplexity <= 15 ? 'A' : maxComplexity <= 25 ? 'B' : maxComplexity <= 35 ? 'C' : 'D';
	statusBarItem.tooltip = new vscode.MarkdownString(`
**File Cognitive Complexity Analysis**

**File:** \`${fileName}\`
**Status:** ${status}
**Complexity Grade:** ${complexityGrade}

**📊 Statistics:**
- Total Functions: ${totalFunctions}
- High Complexity: ${highComplexityFunctions} (${((highComplexityFunctions / totalFunctions) * 100).toFixed(1)}%)
- Average Complexity: ${averageComplexity.toFixed(1)}
- Max Complexity: ${maxComplexity}

**🎯 Recommendations:**
${highComplexityFunctions === 0
			? '✅ Well-structured code with good maintainability'
			: `⚠️ ${highComplexityFunctions} function${highComplexityFunctions === 1 ? '' : 's'} need${highComplexityFunctions === 1 ? 's' : ''} refactoring`}

*Click to run analysis*
	`);
	statusBarItem.tooltip.isTrusted = true;
	statusBarItem.show();
}

function analyzeAndDecorate(editor, complexityModule) {
	if (!editor) {
		updateStatusBar(null);
		return;
	}

	const { document } = editor;
	if (document.languageId !== 'python') {
		updateStatusBar(null);
		return;
	}

	try {
		const code = document.getText();
		const result = complexityModule.code_complexity(code);

		const lowComplexityDecorations = [];
		const highComplexityDecorations = [];

		const lineComplexityMap = new Map();
		const functionComplexityMap = new Map();
		const functionInfoMap = new Map();

		const fileStats = {
			fileName: path.basename(document.fileName),
			totalFunctions: result.functions.length,
			highComplexityFunctions: 0,
			totalComplexity: 0,
			maxComplexity: 0,
			averageComplexity: 0
		};

		result.functions.forEach(func => {
			functionComplexityMap.set(func.line_start, func.complexity);
			functionInfoMap.set(func.line_start, func);

			fileStats.totalComplexity += func.complexity;
			fileStats.maxComplexity = Math.max(fileStats.maxComplexity, func.complexity);
			if (func.complexity > 15) {
				fileStats.highComplexityFunctions++;
			}

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

		if (fileStats.totalFunctions > 0) {
			fileStats.averageComplexity = fileStats.totalComplexity / fileStats.totalFunctions;
		}

		updateStatusBar(fileStats);

		functionComplexityMap.forEach((complexity, lineNum) => {
			const line = lineNum - 1;
			if (line >= 0 && line < document.lineCount) {
				const lineText = document.lineAt(line);
				const position = new vscode.Position(line, lineText.range.end.character);
				const range = new vscode.Range(position, position);

				const funcInfo = functionInfoMap.get(lineNum);
				const functionName = funcInfo ? funcInfo.name : 'Unknown';

				let functionEndLine = lineNum;
				let functionSize = 1;
				if (funcInfo && funcInfo.line_complexities) {
					const lines = funcInfo.line_complexities.map(lc => lc.line);
					if (lines.length > 0) {
						functionEndLine = Math.max(...lines);
						functionSize = functionEndLine - lineNum + 1;
					}
				}

				let highComplexityLineCount = 0;
				let nestingHotspots = 0;
				let booleanOperatorCount = 0;
				let controlFlowCount = 0;

				if (funcInfo && funcInfo.line_complexities) {
					funcInfo.line_complexities.forEach(lineComp => {
						if (lineComp.complexity > 5) {
							highComplexityLineCount++;
						}

						const lineIndex = lineComp.line - 1;
						if (lineIndex >= 0 && lineIndex < document.lineCount) {
							const lineText = document.lineAt(lineIndex).text.toLowerCase();

							const leadingSpaces = document.lineAt(lineIndex).text.length - document.lineAt(lineIndex).text.trimStart().length;
							const nestingLevel = Math.floor(leadingSpaces / 4);
							if (nestingLevel > 2) {
								nestingHotspots++;
							}

							const andMatches = lineText.match(/\band\b/g) || [];
							const orMatches = lineText.match(/\bor\b/g) || [];
							booleanOperatorCount += andMatches.length + orMatches.length;

							if (lineText.includes('if ') || lineText.includes('elif ') ||
								lineText.includes('for ') || lineText.includes('while ') ||
								lineText.includes('try:') || lineText.includes('except ')) {
								controlFlowCount++;
							}
						}
					});
				}

				let hoverMessage = `**Function: \`${functionName}\`**\n\n`;
				hoverMessage += `**Location:** Lines ${lineNum}-${functionEndLine} (${functionSize} lines)\n\n`;
				hoverMessage += `**Cognitive Complexity:** ${complexity}\n\n`;

				if (funcInfo && funcInfo.line_complexities && funcInfo.line_complexities.length > 0) {
					hoverMessage += `**Complexity Analysis:**\n`;
					hoverMessage += `- ${controlFlowCount} control flow structures\n`;
					hoverMessage += `- ${booleanOperatorCount} boolean operators\n`;
					hoverMessage += `- ${nestingHotspots} deeply nested lines (>2 levels)\n`;
					hoverMessage += `- ${highComplexityLineCount} high-complexity lines\n\n`;
				}

				if (complexity <= 15) {
					hoverMessage += `**Status:** Low complexity - Well-structured and maintainable\n\n`;
					hoverMessage += `**Recommendation:** Keep it simple and focused on a single responsibility`;
				} else {
					const totalComplexityLines = funcInfo && funcInfo.line_complexities ? funcInfo.line_complexities.length : 0;
					const averageComplexityPerLine = totalComplexityLines > 0 ? (complexity / totalComplexityLines).toFixed(1) : 0;


					const isAccumulativeComplexity = highComplexityLineCount < 3 && totalComplexityLines > 8 && averageComplexityPerLine <= 2.5;

					if (isAccumulativeComplexity) {
						hoverMessage += `**Status:** Accumulative complexity - Many simple operations\n\n`;
						hoverMessage += `**Pattern:** ${totalComplexityLines} lines with avg ${averageComplexityPerLine} complexity each\n\n`;
						hoverMessage += `**Refactoring Strategy:**\n\n`;
						hoverMessage += `- **Function decomposition**: Break into smaller, focused functions\n`;
						hoverMessage += `- **Extract logical groups**: Group related operations together\n`;
						hoverMessage += `- **Use helper functions**: Create utility functions for repeated patterns\n`;
						hoverMessage += `- **Apply Single Responsibility**: Each function should do one thing well\n`;
						hoverMessage += `- **Consider data structures**: Use dictionaries/classes to reduce branching\n`;
						hoverMessage += `- **Sequential refactoring**: Extract 3-5 lines at a time into helpers\n\n`;
						hoverMessage += `**Why this matters:** While individual lines are simple, the cognitive load of tracking many operations makes the function hard to understand and maintain.`;
					} else {
						hoverMessage += `**Status:** High complexity - Urgent refactoring needed\n\n`;
						hoverMessage += `**Refactoring Priorities:**\n\n`;

						if (nestingHotspots > 0) {
							hoverMessage += `- **Reduce nesting** (${nestingHotspots} deep levels): Use early returns, guard clauses\n`;
						}
						if (booleanOperatorCount > 5) {
							hoverMessage += `- **Simplify boolean logic** (${booleanOperatorCount} operators): Extract conditions to variables\n`;
						}
						if (controlFlowCount > 8) {
							hoverMessage += `- **Break up control flow** (${controlFlowCount} structures): Split into smaller functions\n`;
						}
						if (functionSize > 50) {
							hoverMessage += `- **Reduce function size** (${functionSize} lines): Extract logical chunks\n`;
						}

						hoverMessage += `- **Extract nested functions**: Move complex logic to separate functions\n`;
						hoverMessage += `- **Use design patterns**: Consider Strategy, State, or Command patterns\n`;
						hoverMessage += `- **Target hotspots**: Focus on lines with complexity >5 first`;
					}
				}

				hoverMessage += `\n\n**About Cognitive Complexity:**\n\n`;
				hoverMessage += `Measures how difficult code is to understand. Key factors:\n\n`;
				hoverMessage += `- **Base cost**: +1 for each if/for/while/try/except\n`;
				hoverMessage += `- **Nesting penalty**: +1 per nesting level\n`;
				hoverMessage += `- **Boolean operations**: +1 per and/or operator\n`;
				hoverMessage += `- **Structural complexity**: Nested functions, exception handlers\n\n`;
				hoverMessage += `**Formula:** Base + Nesting + Boolean ops + Structural elements`;

				const decoration = {
					range,
					renderOptions: { after: { contentText: `ƒ ${complexity}` } },
					hoverMessage: new vscode.MarkdownString(hoverMessage)
				};
				decoration.hoverMessage.isTrusted = true; // Allow links

				if (complexity > 15) {
					highComplexityDecorations.push(decoration);
				} else if (complexity > 0) {
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
				const lineText = document.lineAt(line);
				const position = new vscode.Position(line, lineText.range.end.character);
				const range = new vscode.Range(position, position);

				let parentFunction = null;
				for (const [, funcInfo] of functionInfoMap) {
					if (funcInfo.line_complexities) {
						const hasThisLine = funcInfo.line_complexities.some(lc => lc.line === lineNum);
						if (hasThisLine) {
							parentFunction = funcInfo;
							break;
						}
					}
				}

				const codeText = lineText.text.trim().toLowerCase();
				let complexityReason = '';
				let suggestions = '';
				let complexityFactors = [];

				if (codeText.includes('if ') || codeText.includes('elif ') || codeText.includes('else:')) {
					complexityReason = 'Conditional branching';
					complexityFactors.push('Base +1 for branching');
					if (codeText.includes('and ') || codeText.includes('or ')) {
						complexityFactors.push('Additional +1 per boolean operator');
					}

					const leadingSpaces = lineText.text.length - lineText.text.trimStart().length;
					const nestingLevel = Math.floor(leadingSpaces / 4);
					if (nestingLevel > 0) {
						complexityFactors.push(`Nesting penalty: +${nestingLevel}`);
					}
					suggestions = '**Priority Fixes:**\n\n';
					suggestions += '- **Reduce nesting**: Use early returns/guard clauses\n';
					suggestions += '- **Simplify conditions**: Extract complex boolean logic\n';
					suggestions += '- **Split conditions**: Break `and`/`or` chains into variables\n';
					suggestions += '- **Use match statement**: Replace long if/elif chains with cases';
				} else if (codeText.includes('for ') || codeText.includes('while ')) {
					complexityReason = 'Loop iteration';
					complexityFactors.push('Base +1 for loop');
					const leadingSpaces = lineText.text.length - lineText.text.trimStart().length;
					const nestingLevel = Math.floor(leadingSpaces / 4);
					if (nestingLevel > 0) {
						complexityFactors.push(`Nesting penalty: +${nestingLevel}`);
					}
					if (codeText.includes('while ') && (codeText.includes('and ') || codeText.includes('or '))) {
						complexityFactors.push('Additional +1 per boolean operator in condition');
					}
					suggestions = '**Priority Fixes:**\n\n';
					suggestions += '- **Extract loop body**: Move complex logic to separate functions\n';
					suggestions += '- **Use comprehensions**: Replace simple loops with list/dict comprehensions\n';
					suggestions += '- **Built-in functions**: Use `any()`, `all()`, `map()`, `filter()`\n';
					suggestions += '- **Avoid nested loops**: Consider itertools or data structure changes';
				} else if (codeText.includes('try:') || codeText.includes('except ') || codeText.includes('finally:')) {
					complexityReason = 'Exception handling';
					if (codeText.includes('except ')) {
						complexityFactors.push('Base +1 per exception handler');
					}
					suggestions = '**Priority Fixes:**\n\n';
					suggestions += '- **Narrow try blocks**: Only wrap code that can actually fail\n';
					suggestions += '- **Specific exceptions**: Catch specific exception types\n';
					suggestions += '- **Context managers**: Use `with` statements when possible\n';
					suggestions += '- **Reduce handlers**: Combine similar exception handling logic';
				} else if (codeText.includes('and ') || codeText.includes('or ')) {
					complexityReason = 'Boolean operators';
					const andCount = (codeText.match(/\band\b/g) || []).length;
					const orCount = (codeText.match(/\bor\b/g) || []).length;
					complexityFactors.push(`+${andCount + orCount} for boolean operators`);
					suggestions = '**Priority Fixes:**\n\n';
					suggestions += '- **Extract conditions**: Move complex logic to descriptive variables\n';
					suggestions += '- **Use parentheses**: Clarify operator precedence\n';
					suggestions += '- **Apply De Morgan\'s laws**: Simplify negated conditions\n';
					suggestions += '- **Short-circuit evaluation**: Order conditions by likelihood';
				} else if (codeText.includes('with ')) {
					complexityReason = 'Context manager';
					complexityFactors.push('Boolean operations in context expressions');
					suggestions = '**Priority Fixes:**\n\n';
					suggestions += '- **Simplify context expressions**: Extract complex resource acquisition\n';
					suggestions += '- **Multiple contexts**: Use single `with` for multiple resources\n';
					suggestions += '- **Custom context managers**: Create reusable context managers';
				} else if (codeText.includes('return ') || codeText.includes('raise ') || codeText.includes('assert ')) {
					complexityReason = 'Statement with expressions';
					complexityFactors.push('Boolean operations in expression');
					suggestions = '**Priority Fixes:**\n\n';
					suggestions += '- **Simplify expressions**: Extract complex calculations\n';
					suggestions += '- **Use intermediate variables**: Break down complex expressions\n';
					suggestions += '- **Reduce boolean complexity**: Simplify logical operations';
				} else if (codeText.includes('=') && !codeText.includes('==') && !codeText.includes('!=')) {
					complexityReason = 'Assignment with complex expression';
					complexityFactors.push('Boolean operations in assigned value');
					suggestions = '**Priority Fixes:**\n\n';
					suggestions += '- **Extract calculations**: Move complex logic to separate lines\n';
					suggestions += '- **Use intermediate variables**: Break down complex expressions\n';
					suggestions += '- **Simplify right-hand side**: Reduce boolean operations';
				} else {
					complexityReason = 'Control flow structure';
					complexityFactors.push('Contributing to overall complexity');
					suggestions = '**Priority Fixes:**\n\n';
					suggestions += '- **Simplify logic**: Reduce complexity on this line\n';
					suggestions += '- **Extract to function**: Move complex logic to helper function';
				}

				let hoverMessage = `**Line Complexity: +${complexity}**\n\n`;
				hoverMessage += `**Line ${lineNum}:** \`${lineText.text.trim()}\`\n\n`;

				if (parentFunction) {
					hoverMessage += `**Parent Function:** \`${parentFunction.name}\`\n\n`;
				}

				hoverMessage += `**Complexity Source:** ${complexityReason}\n\n`;
				if (complexityFactors.length > 0) {
					hoverMessage += `**Complexity Breakdown:**\n\n`;
					complexityFactors.forEach(factor => {
						hoverMessage += `- ${factor}\n`;
					});
					hoverMessage += `\n`;
				}

				if (complexity <= 5) {
					hoverMessage += `**Impact:** Low - Minor complexity addition\n\n`;
					hoverMessage += `This line contributes minimally to overall complexity`;
				} else {
					hoverMessage += `**Impact:** High - Significant complexity hotspot\n\n`;
					hoverMessage += `${suggestions}`;
				}

				hoverMessage += `\n\n**Understanding the Calculation:**\n\n`;
				hoverMessage += `Complexity = Base cost + Nesting penalty + Boolean operations\n\n`;
				hoverMessage += `- **Nesting**: Each level adds +1 to base cost\n`;
				hoverMessage += `- **Boolean ops**: Each \`and\`/\`or\` adds +1\n`;
				hoverMessage += `- **Control flow**: if/for/while/try add base +1\n\n`;
				hoverMessage += `**Focus Strategy:** Target high-complexity lines in nested contexts first`;

				const decoration = {
					range,
					renderOptions: { after: { contentText: `+${complexity}` } },
					hoverMessage: new vscode.MarkdownString(hoverMessage)
				};
				decoration.hoverMessage.isTrusted = true; // Allow links

				if (complexity > 5) {
					highComplexityDecorations.push(decoration);
				} else {
					lowComplexityDecorations.push(decoration);
				}
			}
		});

		editor.setDecorations(lowComplexityDecorationType, lowComplexityDecorations);
		editor.setDecorations(highComplexityDecorationType, highComplexityDecorations);
	} catch (e) {
		console.error(`Error analyzing complexity: ${e.message}`);
		allDecorationTypes.forEach(decorationType => {
			editor.setDecorations(decorationType, []);
		});
		updateStatusBar(null);
	}
}

/**
 * @param {vscode.ExtensionContext} context
 */
async function activate(context) {
	const wasmJsPath = path.join(context.extensionPath, 'wasm', 'complexipy_wasm.js');
	const complexityModule = require(wasmJsPath);
	const wasmBgPath = path.join(context.extensionPath, 'wasm', 'complexipy_wasm_bg.wasm');

	try {
		const wasmBuffer = await fs.readFile(wasmBgPath);
		const wasmModule = await WebAssembly.compile(wasmBuffer);
		await complexityModule.default({ module_or_path: wasmModule });
	} catch (err) {
		vscode.window.showErrorMessage('Failed to load complexipy WASM module. Please check the console for details.');
		console.error("Failed to load/initialize complexipy WASM module:", err);
		return;
	}

	statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
	statusBarItem.command = 'complexipy.calculateCognitiveComplexity';
	context.subscriptions.push(statusBarItem);

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

	let disposable = vscode.commands.registerCommand('complexipy.calculateCognitiveComplexity', () => {
		if (activeEditor && activeEditor.document.languageId === 'python') {
			analyzeAndDecorate(activeEditor, complexityModule);
		} else if (activeEditor) {
			vscode.window.showInformationMessage('Please open a Python file to analyze.');
		} else {
			vscode.window.showInformationMessage('No active editor found.');
		}
	});

	context.subscriptions.push(disposable);
}

function deactivate() {
	if (statusBarItem) {
		statusBarItem.dispose();
	}
}

module.exports = {
	activate,
	deactivate
}
