const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { serverCommand, findExecutable, countOverThreshold, statusText, functionBand, lineBand, badges } = require('../lib');

test('serverCommand falls back to complexipy on PATH', () => {
	for (const value of [undefined, null, '', '   ']) {
		assert.deepStrictEqual(serverCommand(value), { command: 'complexipy', args: ['lsp'] });
	}
});

test('serverCommand uses the configured path', () => {
	assert.deepStrictEqual(serverCommand(' /opt/bin/complexipy '), { command: '/opt/bin/complexipy', args: ['lsp'] });
});

test('countOverThreshold counts only complexipy complexity warnings', () => {
	const diagnostics = [
		{ source: 'complexipy', code: 'cognitive-complexity' },
		{ source: 'complexipy', code: { value: 'cognitive-complexity', target: {} } },
		{ source: 'complexipy', code: 'complexipy-parse-error' },
		{ source: 'ruff', code: 'cognitive-complexity' },
		{ source: 'complexipy' }
	];
	assert.strictEqual(countOverThreshold(diagnostics), 2);
	assert.strictEqual(countOverThreshold([]), 0);
});

test('statusText shows a check at zero and the count otherwise', () => {
	assert.strictEqual(statusText(0), '$(check) complexipy');
	assert.strictEqual(statusText(1), '$(warning) 1 over threshold');
	assert.strictEqual(statusText(3), '$(warning) 3 over threshold');
});

test('findExecutable resolves commands on PATH and absolute paths', t => {
	const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'complexipy-'));
	t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
	const binary = path.join(dir, 'complexipy');
	fs.writeFileSync(binary, '');
	fs.chmodSync(binary, 0o755);
	const plain = path.join(dir, 'plain');
	fs.writeFileSync(plain, '');
	fs.chmodSync(plain, 0o644);

	assert.strictEqual(findExecutable('complexipy', { PATH: `/nonexistent:${dir}` }, 'darwin'), binary);
	assert.strictEqual(findExecutable('complexipy', { PATH: '/nonexistent' }, 'darwin'), undefined);
	assert.strictEqual(findExecutable('complexipy', {}, 'darwin'), undefined);
	assert.strictEqual(findExecutable(binary, {}, 'darwin'), binary);
	assert.strictEqual(findExecutable(path.join(dir, 'missing'), {}, 'darwin'), undefined);
	assert.strictEqual(findExecutable(plain, {}, 'darwin'), undefined);
	assert.strictEqual(findExecutable(dir, {}, 'darwin'), undefined);
});

test('functionBand is green up to half the limit, yellow up to it, red above it', () => {
	const bands = [7, 8, 15, 16].map(complexity => functionBand(complexity, 15));
	assert.deepStrictEqual(bands, ['green', 'yellow', 'yellow', 'red']);
	assert.strictEqual(functionBand(0, 0), 'green');
});

test('lineBand uses the warning and error cut-offs', () => {
	assert.deepStrictEqual([1, 2, 3, 4, 5, 9].map(increment => lineBand(increment, 3, 5)), ['green', 'green', 'yellow', 'yellow', 'red', 'red']);
	assert.deepStrictEqual([3, 4, 6].map(increment => lineBand(increment, 6, 4)), ['green', 'red', 'red']);
});

test('badges lists a function badge and one badge per line increment', () => {
	const result = {
		version: 1,
		maxComplexityAllowed: 2,
		functions: [
			{ name: 'light', complexity: 0, line: 0, lines: [] },
			{ name: 'heavy', complexity: 3, line: 4, lines: [{ line: 5, complexity: 1 }, { line: 6, complexity: 2 }] }
		]
	};
	assert.deepStrictEqual(badges(result, 2, 3), [
		{ line: 0, text: 'ƒ 0', band: 'green' },
		{ line: 4, text: 'ƒ 3', band: 'red' },
		{ line: 5, text: '+1', band: 'green' },
		{ line: 6, text: '+2', band: 'yellow' }
	]);
});
