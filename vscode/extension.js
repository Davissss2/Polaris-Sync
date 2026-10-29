const vscode = require("vscode");
const { execFile } = require("child_process");
const fs = require("fs");
const path = require("path");

const SECTION = "polaris";
const PROJECT_FILE = "default.project.json";

let statusBar;
let output;
let timer;

function config() {
	return vscode.workspace.getConfiguration(SECTION);
}

function executable() {
	return config().get("executable", "polaris");
}

function quote(value) {
	return process.platform === "win32" ? `"${value}"` : `'${value}'`;
}

// `list --json` reports the address, not the port, so it is read from there
function portOf(session) {
	const address = String(session.address || "");
	const match = address.match(/:(\d+)\/?$/);

	return match ? match[1] : String(session.id);
}

function run(args, cwd) {
	return new Promise((resolve) => {
		execFile(
			executable(),
			args,
			{ cwd, windowsHide: true, maxBuffer: 8 * 1024 * 1024 },
			(error, stdout, stderr) => {
				resolve({
					ok: !error,
					code: error && typeof error.code === "number" ? error.code : 0,
					stdout: String(stdout || ""),
					stderr: String(stderr || ""),
					message: error ? error.message : "",
				});
			}
		);
	});
}

async function sessions() {
	const result = await run(["list", "--json"]);

	if (!result.ok) {
		return [];
	}

	try {
		const parsed = JSON.parse(result.stdout);
		return Array.isArray(parsed) ? parsed : [];
	} catch {
		return [];
	}
}

function workspaceRoot() {
	const folders = vscode.workspace.workspaceFolders;
	return folders && folders.length ? folders[0].uri.fsPath : undefined;
}

// Looks for the folder that holds default.project.json, which is what `serve` takes
function findProjects() {
	const folders = vscode.workspace.workspaceFolders || [];
	const found = [];

	for (const folder of folders) {
		const root = folder.uri.fsPath;

		if (fs.existsSync(path.join(root, PROJECT_FILE))) {
			found.push(root);
			continue;
		}

		let entries = [];

		try {
			entries = fs.readdirSync(root, { withFileTypes: true });
		} catch {
			continue;
		}

		for (const entry of entries) {
			if (!entry.isDirectory() || entry.name.startsWith(".") || entry.name === "node_modules") {
				continue;
			}

			const candidate = path.join(root, entry.name);

			if (fs.existsSync(path.join(candidate, PROJECT_FILE))) {
				found.push(candidate);
			}
		}
	}

	return found;
}

async function pickProject() {
	const override = config().get("projectPath", "");

	if (override) {
		const root = workspaceRoot();
		return root ? path.resolve(root, override) : override;
	}

	const projects = findProjects();

	if (projects.length === 1) {
		return projects[0];
	}

	if (projects.length === 0) {
		const choice = await vscode.window.showWarningMessage(
			"No default.project.json found in this workspace.",
			"New Project"
		);

		if (choice === "New Project") {
			await vscode.commands.executeCommand("polaris.init");
		}

		return undefined;
	}

	const picked = await vscode.window.showQuickPick(
		projects.map((project) => ({
			label: vscode.workspace.asRelativePath(project),
			description: project,
			project,
		})),
		{ placeHolder: "Which Polaris project?" }
	);

	return picked ? picked.project : undefined;
}

function terminalAt(cwd) {
	const terminal = vscode.window.createTerminal({ name: "Polaris", cwd });
	terminal.show(true);

	return terminal;
}

function commandLine(exe, rest) {
	return [quote(exe), ...rest.map(quote)].join(" ");
}

async function start() {
	const project = await pickProject();

	if (!project) {
		return;
	}

	const running = await sessions();
	const match = running.find(
		(session) => session.project && path.resolve(session.project) === path.resolve(path.join(project, PROJECT_FILE))
	);

	if (match) {
		vscode.window.showInformationMessage(
			`Polaris is already serving ${vscode.workspace.asRelativePath(project)} on port ${portOf(match)}.`
		);
		refresh();

		return;
	}

	const exe = executable();
	const terminal = terminalAt(project);
	terminal.sendText(commandLine(exe, ["serve", project]));

	vscode.window.showInformationMessage(
		`Polaris is starting for ${vscode.workspace.asRelativePath(project)}. Open Roblox Studio to connect.`
	);

	setTimeout(refresh, 2500);
}

async function stop() {
	const running = await sessions();

	if (running.length === 0) {
		vscode.window.showInformationMessage("No Polaris session is running.");

		return;
	}

	const label =
		running.length === 1
			? `Stop the session on port ${portOf(running[0])}?`
			: `Stop ${running.length} Polaris sessions?`;

	const confirmed = await vscode.window.showWarningMessage(label, { modal: true }, "Stop");

	if (confirmed !== "Stop") {
		return;
	}

	const result = await run(["stop", "--all", "--yes"]);

	if (result.ok) {
		vscode.window.showInformationMessage("Polaris sessions stopped.");
	} else {
		vscode.window.showErrorMessage(`Could not stop Polaris: ${result.stderr || result.message}`);
	}

	refresh();
}

async function build() {
	const project = await pickProject();

	if (!project) {
		return;
	}

	const terminal = terminalAt(project);
	terminal.sendText(commandLine(executable(), ["build", project]));
}

async function init() {
	const root = workspaceRoot();

	if (!root) {
		vscode.window.showErrorMessage("Open a folder before creating a Polaris project.");

		return;
	}

	const name = await vscode.window.showInputBox({
		prompt: "Project name",
		value: "game",
		validateInput: (value) => (value.trim() ? undefined : "Give the project a name"),
	});

	if (!name) {
		return;
	}

	const terminal = terminalAt(root);
	terminal.sendText(commandLine(executable(), ["init", name.trim(), "--yes"]));

	vscode.window.showInformationMessage(
		`Creating ${name.trim()}. When it finishes, run "Polaris: Start Server" and open Studio.`
	);
}

async function installPlugin() {
	const result = await run(["plugin", "install", "--yes"]);

	const text = (result.stdout + result.stderr).trim();

	if (!result.ok) {
		vscode.window.showErrorMessage(`Could not install the plugin: ${text || result.message}`);

		return;
	}

	vscode.window
		.showInformationMessage("Studio plugin installed. Restart Roblox Studio to load it.", "Show details")
		.then((choice) => {
			if (choice === "Show details") {
				output.clear();
				output.appendLine(text);
				output.show();
			}
		});

	output.clear();
	output.appendLine(text);
}

async function sessionInfo() {
	const running = await sessions();

	output.clear();

	if (running.length === 0) {
		output.appendLine("No Polaris session is running.");

		const choice = await vscode.window.showInformationMessage("No Polaris session is running.", "Start Server");

		if (choice === "Start Server") {
			await start();
		} else {
			output.show();
		}

		return;
	}

	for (const session of running) {
		output.appendLine(`session ${session.id}`);
		output.appendLine(`  project ${session.project}`);
		output.appendLine(`  address ${session.address}`);
		output.appendLine(`  pid     ${session.pid}`);
		output.appendLine(`  health  ${session.healthy ? "ok" : "unhealthy"}`);
		output.appendLine("");
	}

	output.show();
}

async function openMenu() {
	const running = await sessions();
	const project = findProjects()[0];
	const state = running.length
		? `$(radio-tower) ${running.length} session${running.length > 1 ? "s" : ""} running`
		: "$(circle-slash) no session";

	const items = [
		{
			label: "$(play) Start server",
			description: project ? vscode.workspace.asRelativePath(project) : "no project found",
			action: start,
		},
		{
			label: "$(debug-stop) Stop server",
			description: state,
			action: stop,
		},
		{
			label: "$(package) Build place",
			description: "polaris build",
			action: build,
		},
		{
			label: "$(add) New project",
			description: "polaris init",
			action: init,
		},
		{
			label: "$(plug) Install Studio plugin",
			description: "polaris plugin install",
			action: installPlugin,
		},
		{
			label: "$(list-unordered) Session info",
			description: state,
			action: sessionInfo,
		},
		{
			label: "$(gear) Settings",
			description: "executable, project path, refresh interval",
			action: () => vscode.commands.executeCommand("workbench.action.openSettings", "polaris"),
		},
	];

	const picked = await vscode.window.showQuickPick(items, { placeHolder: "Polaris" });

	if (picked) {
		await picked.action();
	}
}

async function refresh() {
	if (!statusBar) {
		return;
	}

	const running = await sessions();

	if (running.length === 0) {
		statusBar.text = "$(circle-slash) Polaris";
		statusBar.tooltip = "No Polaris session running. Click to start one.";
		statusBar.backgroundColor = undefined;

		return;
	}

	const unhealthy = running.some((session) => !session.healthy);
	const ports = running.map(portOf).join(", ");

	statusBar.text = `$(radio-tower) Polaris ${ports}`;
	statusBar.tooltip = running
		.map(
			(session) =>
				`${session.project}\n${session.address}${session.healthy ? "" : " (unhealthy)"}\npid ${session.pid}`
		)
		.join("\n\n");

	statusBar.backgroundColor = unhealthy
		? new vscode.ThemeColor("statusBarItem.warningBackground")
		: undefined;
}

function activate(context) {
	output = vscode.window.createOutputChannel("Polaris");

	statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
	statusBar.command = "polaris.openMenu";
	statusBar.show();

	context.subscriptions.push(
		output,
		statusBar,
		vscode.commands.registerCommand("polaris.openMenu", openMenu),
		vscode.commands.registerCommand("polaris.start", start),
		vscode.commands.registerCommand("polaris.stop", stop),
		vscode.commands.registerCommand("polaris.build", build),
		vscode.commands.registerCommand("polaris.init", init),
		vscode.commands.registerCommand("polaris.installPlugin", installPlugin),
		vscode.commands.registerCommand("polaris.sessionInfo", sessionInfo),
		vscode.workspace.onDidChangeConfiguration((event) => {
			if (event.affectsConfiguration(SECTION)) {
				scheduleRefresh();
				refresh();
			}
		})
	);

	scheduleRefresh();
	refresh();
}

function scheduleRefresh() {
	if (timer) {
		clearInterval(timer);
		timer = undefined;
	}

	const interval = Number(config().get("refreshInterval", 3000));

	if (interval > 0) {
		timer = setInterval(refresh, Math.max(1000, interval));
	}
}

function deactivate() {
	if (timer) {
		clearInterval(timer);
		timer = undefined;
	}
}

module.exports = { activate, deactivate };
