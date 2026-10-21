const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { serverCommand, findExecutable, countOverThreshold, statusText } = require('../lib');

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

test('findExecutable resolves a relative path against the workspace root only', t => {
	const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'complexipy-workspace-'));
	t.after(() => fs.rmSync(workspace, { recursive: true, force: true }));
	fs.mkdirSync(path.join(workspace, 'tools'));
	const binary = path.join(workspace, 'tools', 'complexipy');
	fs.writeFileSync(binary, '');
	fs.chmodSync(binary, 0o755);

	assert.strictEqual(findExecutable('./tools/complexipy', {}, 'darwin', workspace), binary);
	assert.strictEqual(findExecutable('tools/complexipy', {}, 'darwin', workspace), binary);
	assert.strictEqual(findExecutable('./tools/complexipy', {}, 'darwin'), undefined);
});

test('findExecutable skips relative PATH entries', t => {
	const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'complexipy-workspace-'));
	t.after(() => fs.rmSync(workspace, { recursive: true, force: true }));
	const binary = path.join(workspace, 'complexipy');
	fs.writeFileSync(binary, '');
	fs.chmodSync(binary, 0o755);
	const previous = process.cwd();
	process.chdir(workspace);
	t.after(() => process.chdir(previous));

	assert.strictEqual(findExecutable('complexipy', { PATH: '.' }, 'darwin', workspace), undefined);
	assert.strictEqual(findExecutable('complexipy', { PATH: `.:${workspace}` }, 'darwin', workspace), binary);
});
