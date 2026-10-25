const fs = require('fs');
const path = require('path');

const DEFAULT_COMMAND = 'complexipy';
const DIAGNOSTIC_SOURCE = 'complexipy';
const DIAGNOSTIC_CODE = 'cognitive-complexity';

function serverCommand(configuredPath) {
	const path = typeof configuredPath === 'string' ? configuredPath.trim() : '';
	return { command: path || DEFAULT_COMMAND, args: ['lsp'] };
}

function isExecutable(file) {
	try {
		fs.accessSync(file, fs.constants.X_OK);
		return fs.statSync(file).isFile();
	} catch {
		return false;
	}
}

function findExecutable(command, env = process.env, platform = process.platform) {
	if (path.isAbsolute(command) || command.includes('/') || command.includes('\\')) {
		return isExecutable(command) ? command : undefined;
	}
	const extensions = platform === 'win32' ? (env.PATHEXT || '.EXE;.CMD;.BAT').split(';') : [''];
	const delimiter = platform === 'win32' ? ';' : ':';
	for (const dir of (env.PATH || '').split(delimiter).filter(Boolean)) {
		for (const extension of extensions) {
			const candidate = path.join(dir, command + extension);
			if (isExecutable(candidate)) {
				return candidate;
			}
		}
	}
	return undefined;
}

function diagnosticCode(diagnostic) {
	const code = diagnostic.code;
	return code !== null && typeof code === 'object' ? code.value : code;
}

function countOverThreshold(diagnostics) {
	return diagnostics.filter(diagnostic =>
		diagnostic.source === DIAGNOSTIC_SOURCE && diagnosticCode(diagnostic) === DIAGNOSTIC_CODE
	).length;
}

function statusText(count) {
	return count === 0 ? '$(check) complexipy' : `$(warning) ${count} over threshold`;
}

function functionBand(complexity, maxComplexityAllowed) {
	if (complexity > maxComplexityAllowed) {
		return 'red';
	}
	return complexity * 2 > maxComplexityAllowed ? 'yellow' : 'green';
}

function lineBand(increment, warningAt, errorAt) {
	if (increment >= errorAt) {
		return 'red';
	}
	return increment >= warningAt ? 'yellow' : 'green';
}

function badges(result, warningAt, errorAt) {
	return result.functions.flatMap(fn => [
		{ line: fn.line, text: `ƒ ${fn.complexity}`, band: functionBand(fn.complexity, result.maxComplexityAllowed) },
		...fn.lines.map(entry => ({
			line: entry.line,
			text: `+${entry.complexity}`,
			band: lineBand(entry.complexity, warningAt, errorAt)
		}))
	]);
}

module.exports = {
	serverCommand,
	findExecutable,
	countOverThreshold,
	statusText,
	functionBand,
	lineBand,
	badges
};
