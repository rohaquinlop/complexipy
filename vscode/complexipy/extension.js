const vscode = require('vscode');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');
const { serverCommand, findExecutable, countOverThreshold, statusText } = require('./lib');

const INSTALL_HINT = 'Install it with "uv tool install complexipy" or set complexipy.server.path.';

let client;
let statusBarItem;
let serverFailed = false;
let pendingRestart = Promise.resolve();

function createClient(command, args) {
	const serverOptions = { command, args, transport: TransportKind.stdio };
	const clientOptions = {
		documentSelector: [
			{ scheme: 'file', language: 'python' },
			{ scheme: 'untitled', language: 'python' }
		]
	};
	return new LanguageClient('complexipy', 'complexipy', serverOptions, clientOptions);
}

async function startClient() {
	const { command, args } = serverCommand(vscode.workspace.getConfiguration('complexipy').get('server.path'));
	const executable = findExecutable(command);
	serverFailed = executable === undefined;
	if (serverFailed) {
		vscode.window.showErrorMessage(`complexipy: cannot find "${command}". ${INSTALL_HINT}`);
	} else {
		client = createClient(executable, args);
		try {
			await client.start();
		} catch (err) {
			serverFailed = true;
			console.error('complexipy: language server failed to start:', err);
		}
	}
	updateStatusBar();
}

async function stopClient() {
	if (client) {
		const stopping = client;
		client = undefined;
		await stopping.stop().catch(() => undefined);
	}
}

function restartClient() {
	pendingRestart = pendingRestart.then(stopClient).then(startClient);
	return pendingRestart;
}

function updateStatusBar() {
	const editor = vscode.window.activeTextEditor;
	if (!editor || editor.document.languageId !== 'python') {
		statusBarItem.hide();
		return;
	}
	if (serverFailed) {
		statusBarItem.text = '$(error) complexipy';
		statusBarItem.tooltip = `The complexipy language server is not running. ${INSTALL_HINT}`;
	} else {
		statusBarItem.text = statusText(countOverThreshold(vscode.languages.getDiagnostics(editor.document.uri)));
		statusBarItem.tooltip = 'Functions over max-complexity-allowed. Click to open the Problems panel.';
	}
	statusBarItem.show();
}

async function activate(context) {
	statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
	statusBarItem.command = 'workbench.actions.view.problems';
	context.subscriptions.push(statusBarItem);

	context.subscriptions.push(
		vscode.window.onDidChangeActiveTextEditor(updateStatusBar),
		vscode.languages.onDidChangeDiagnostics(event => {
			const editor = vscode.window.activeTextEditor;
			if (editor && event.uris.some(uri => uri.toString() === editor.document.uri.toString())) {
				updateStatusBar();
			}
		}),
		vscode.workspace.onDidChangeConfiguration(event => {
			if (event.affectsConfiguration('complexipy.server.path')) {
				restartClient();
			}
		})
	);

	await restartClient();
}

function deactivate() {
	return stopClient();
}

module.exports = {
	activate,
	deactivate
};
