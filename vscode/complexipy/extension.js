const vscode = require('vscode');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');
const { serverCommand, findExecutable, countOverThreshold, statusText, withinTimeout, badges } = require('./lib');

const INSTALL_HINT = 'Install it with "uv tool install complexipy" or set complexipy.server.path.';
const START_TIMEOUT_MS = 30000;
const LINE_COMPLEXITIES = 'complexipy/lineComplexities';
const METHOD_NOT_FOUND = -32601;
const DECORATIONS_MIN_VERSION = '8.2.0';
const REFRESH_DELAY_MS = 150;

const BADGE_STYLE = {
	margin: '0 0.5em',
	textDecoration: 'none; border-radius: 3px; padding: 0.1em 0.4em;'
};

const BAND_COLORS = {
	green: {
		dark: { backgroundColor: 'rgba(61, 139, 64, 0.8)', color: '#FFFFFF' },
		light: { backgroundColor: 'rgba(212, 237, 218, 0.8)', color: '#155724' }
	},
	yellow: {
		dark: { backgroundColor: 'rgba(160, 115, 10, 0.8)', color: '#FFFFFF' },
		light: { backgroundColor: 'rgba(255, 243, 205, 0.8)', color: '#856404' }
	},
	red: {
		dark: { backgroundColor: 'rgba(166, 51, 51, 0.8)', color: '#FFFFFF' },
		light: { backgroundColor: 'rgba(248, 215, 218, 0.8)', color: '#721C24' }
	}
};

let client;
let statusBarItem;
let outputChannel;
let serverFailed = false;
let decorationsUnsupported = false;
let pendingRestart = Promise.resolve();
let decorationTypes = {};
const refreshTimers = new Map();

function settings() {
	return vscode.workspace.getConfiguration('complexipy');
}

function decorationsActive() {
	return settings().get('display') === 'decorations' && !decorationsUnsupported;
}

function workspaceRoot() {
	const folder = vscode.workspace.workspaceFolders?.[0];
	return folder?.uri.scheme === 'file' ? folder.uri.fsPath : undefined;
}

function createClient(command, args, cwd) {
	const serverOptions = { command, args, transport: TransportKind.stdio, options: cwd === undefined ? {} : { cwd } };
	const clientOptions = {
		documentSelector: [
			{ scheme: 'file', language: 'python' },
			{ scheme: 'untitled', language: 'python' }
		],
		outputChannel,
		middleware: {
			provideInlayHints: (document, range, token, next) => (decorationsActive() ? [] : next(document, range, token))
		}
	};
	return new LanguageClient('complexipy', 'complexipy', serverOptions, clientOptions);
}

async function startClient() {
	const { command, args } = serverCommand(settings().get('server.path'));
	const cwd = workspaceRoot();
	const executable = findExecutable(command, process.env, process.platform, cwd);
	serverFailed = executable === undefined;
	decorationsUnsupported = false;
	if (serverFailed) {
		vscode.window.showErrorMessage(`complexipy: cannot find "${command}". ${INSTALL_HINT}`);
	} else {
		client = createClient(executable, args, cwd);
		try {
			await withinTimeout(client.start(), START_TIMEOUT_MS);
		} catch (err) {
			serverFailed = true;
			console.error('complexipy: language server failed to start:', err);
			outputChannel.appendLine(`The language server failed to start: ${err.message}`);
			vscode.window.showErrorMessage('complexipy: the language server did not start. See the complexipy output for details.');
			await stopClient();
		}
	}
	updateStatusBar();
	refreshVisibleEditors();
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

function createDecorationTypes() {
	return Object.fromEntries(Object.entries(BAND_COLORS).map(([band, colors]) => [
		band,
		vscode.window.createTextEditorDecorationType({
			dark: { after: { ...BADGE_STYLE, ...colors.dark } },
			light: { after: { ...BADGE_STYLE, ...colors.light } }
		})
	]));
}

function clearDecorations(editor) {
	for (const decorationType of Object.values(decorationTypes)) {
		editor.setDecorations(decorationType, []);
	}
}

function applyBadges(editor, result) {
	const document = editor.document;
	const ranges = Object.fromEntries(Object.keys(decorationTypes).map(band => [band, []]));
	const config = settings();
	const warningAt = config.get('decorations.lineWarningAt');
	const errorAt = config.get('decorations.lineErrorAt');
	for (const badge of badges(result, warningAt, errorAt)) {
		if (badge.line < document.lineCount) {
			const end = document.lineAt(badge.line).range.end;
			ranges[badge.band].push({ range: new vscode.Range(end, end), renderOptions: { after: { contentText: badge.text } } });
		}
	}
	for (const [band, decorationType] of Object.entries(decorationTypes)) {
		editor.setDecorations(decorationType, ranges[band]);
	}
}

function refreshInlayHints() {
	const feature = client && client.getFeature('textDocument/inlayHint');
	for (const editor of vscode.window.visibleTextEditors) {
		const provider = feature && feature.getProvider(editor.document);
		if (provider) {
			provider.onDidChangeInlayHints.fire();
		}
	}
}

function handleUnsupported() {
	decorationsUnsupported = true;
	vscode.window.showWarningMessage(
		`complexipy: colored decorations need complexipy ${DECORATIONS_MIN_VERSION} or later. Showing inlay hints instead.`
	);
	for (const editor of vscode.window.visibleTextEditors) {
		clearDecorations(editor);
	}
	refreshInlayHints();
}

async function refreshEditor(editor) {
	const document = editor.document;
	if (document.languageId !== 'python' || !client || !client.isRunning() || !decorationsActive()) {
		clearDecorations(editor);
		return;
	}
	let result;
	try {
		result = await client.sendRequest(LINE_COMPLEXITIES, {
			textDocument: { uri: client.code2ProtocolConverter.asUri(document.uri) }
		});
	} catch (err) {
		if (err && err.code === METHOD_NOT_FOUND) {
			if (!decorationsUnsupported) {
				handleUnsupported();
			}
		} else {
			console.error('complexipy: line complexities request failed:', err);
		}
		return;
	}
	if (decorationsActive() && result.version >= document.version) {
		applyBadges(editor, result);
	}
}

function refreshVisibleEditors() {
	for (const editor of vscode.window.visibleTextEditors) {
		refreshEditor(editor);
	}
}

function refreshDocumentSoon(document) {
	const key = document.uri.toString();
	clearTimeout(refreshTimers.get(key));
	refreshTimers.set(key, setTimeout(() => {
		refreshTimers.delete(key);
		for (const editor of vscode.window.visibleTextEditors) {
			if (editor.document.uri.toString() === key) {
				refreshEditor(editor);
			}
		}
	}, REFRESH_DELAY_MS));
}

async function activate(context) {
	outputChannel = vscode.window.createOutputChannel('complexipy');
	context.subscriptions.push(outputChannel);
	statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
	statusBarItem.command = 'workbench.actions.view.problems';
	decorationTypes = createDecorationTypes();
	context.subscriptions.push(statusBarItem, ...Object.values(decorationTypes));

	context.subscriptions.push(
		vscode.window.onDidChangeActiveTextEditor(updateStatusBar),
		vscode.window.onDidChangeVisibleTextEditors(refreshVisibleEditors),
		vscode.workspace.onDidChangeTextDocument(event => {
			if (event.document.languageId === 'python' && decorationsActive()) {
				refreshDocumentSoon(event.document);
			}
		}),
		vscode.languages.onDidChangeDiagnostics(event => {
			const editor = vscode.window.activeTextEditor;
			if (editor && event.uris.some(uri => uri.toString() === editor.document.uri.toString())) {
				updateStatusBar();
			}
			for (const uri of event.uris) {
				const document = vscode.workspace.textDocuments.find(doc => doc.uri.toString() === uri.toString());
				if (document && document.languageId === 'python' && decorationsActive()) {
					refreshDocumentSoon(document);
				}
			}
		}),
		vscode.workspace.onDidChangeConfiguration(event => {
			if (event.affectsConfiguration('complexipy.server.path')) {
				restartClient();
			} else if (event.affectsConfiguration('complexipy.display') || event.affectsConfiguration('complexipy.decorations')) {
				refreshVisibleEditors();
				refreshInlayHints();
			}
		}),
		vscode.workspace.onDidGrantWorkspaceTrust(restartClient),
		{ dispose: () => refreshTimers.forEach(clearTimeout) }
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
