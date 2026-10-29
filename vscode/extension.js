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

// A session belongs to this window only when its project file lives inside one
// of the open folders, which keeps two editors with two places apart
function belongsToWorkspace(session) {
	if (!session.project) {
		return false;
	}

	const target = path.resolve(session.project).toLowerCase();
	const folders = vscode.workspace.workspaceFolders || [];

	return folders.some((folder) => {
		const root = path.resolve(folder.uri.fsPath).toLowerCase();

		return target === root || target.startsWith(root.endsWith(path.sep) ? root : root + path.sep);
	});
}

async function mySessions() {
	const all = await sessions();

	return all.filter(belongsToWorkspace);
}

function projectName(session) {
	return path.basename(path.dirname(session.project || ""));
}

// Older servers do not report clients at all, so undefined is "unknown" and
// must not be shown as "nothing is connected"
function clientsOf(session) {
	return Array.isArray(session.clients) ? session.clients : undefined;
}

function studioState(session) {
	const clients = clientsOf(session);

	if (clients === undefined) {
		return undefined;
	}

	if (clients.length === 0) {
		return "no Studio";
	}

	return clients.join(", ");
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

	const wanted = path.resolve(path.join(project, PROJECT_FILE)).toLowerCase();
	const running = await mySessions();
	const match = running.find((session) => session.project && path.resolve(session.project).toLowerCase() === wanted);

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
	const mine = await mySessions();
	const others = (await sessions()).length - mine.length;

	if (mine.length === 0) {
		const detail = others
			? ` There ${others === 1 ? "is 1 session" : `are ${others} sessions`} from another window, left alone.`
			: "";

		vscode.window.showInformationMessage(`No Polaris session is running for this workspace.${detail}`);

		return;
	}

	const label =
		mine.length === 1
			? `Stop the session for ${projectName(mine[0])} on port ${portOf(mine[0])}?`
			: `Stop ${mine.length} sessions of this workspace?`;

	const confirmed = await vscode.window.showWarningMessage(label, { modal: true }, "Stop");

	if (confirmed !== "Stop") {
		return;
	}

	// One id per session: `--all` would take down the other editor's server too
	for (const session of mine) {
		const result = await run(["stop", String(session.id), "--yes"]);

		if (!result.ok) {
			vscode.window.showErrorMessage(
				`Could not stop ${projectName(session)}: ${result.stderr || result.message}`
			);
		}
	}

	vscode.window.showInformationMessage(
		mine.length === 1 ? "Polaris session stopped." : `${mine.length} Polaris sessions stopped.`
	);

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
	const all = await sessions();
	const mine = all.filter(belongsToWorkspace);
	const others = all.filter((session) => !belongsToWorkspace(session));

	output.clear();
	output.appendLine(`workspace ${workspaceRoot() || "(none)"}`);
	output.appendLine("");

	if (mine.length === 0) {
		output.appendLine("No Polaris session is running for this workspace.");
		output.appendLine("");
	} else {
		output.appendLine("this workspace:");
		appendSessions(mine);
	}

	if (others.length > 0) {
		output.appendLine("other windows (left alone):");
		appendSessions(others);
	}

	output.show();

	if (mine.length === 0) {
		const choice = await vscode.window.showInformationMessage(
			"No Polaris session is running for this workspace.",
			"Start Server"
		);

		if (choice === "Start Server") {
			await start();
		}
	}
}

function appendSessions(list) {
	for (const session of list) {
		const studio = studioState(session);

		output.appendLine(`  session ${session.id}  (${projectName(session)})`);
		output.appendLine(`    project ${session.project}`);
		output.appendLine(`    address ${session.address}`);
		output.appendLine(`    clients ${studio === undefined ? "unknown" : studio}`);
		output.appendLine(`    pid     ${session.pid}`);
		output.appendLine(`    health  ${session.healthy ? "ok" : "unhealthy"}`);
	}

	output.appendLine("");
}

async function openMenu() {
	const all = await sessions();
	const mine = all.filter(belongsToWorkspace);
	const outside = all.length - mine.length;
	const project = findProjects()[0];
	const state = mine.length
		? mine
				.map((session) => {
					const studio = studioState(session);

					return `${projectName(session)}:${portOf(session)}${studio === "no Studio" ? " (no Studio)" : ""}`;
				})
				.join(", ")
		: "$(circle-slash) none for this workspace";

	const items = [
		{
			label: "$(play) Start server",
			description: project ? vscode.workspace.asRelativePath(project) : "no project found",
			action: start,
		},
		{
			label: "$(debug-stop) Stop server",
			description: mine.length ? state : outside ? `${outside} running elsewhere, left alone` : "nothing to stop",
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

	// Nothing to show when this window is not a Polaris project: no status bar
	// in unrelated workspaces
	if (findProjects().length === 0) {
		statusBar.hide();

		return;
	}

	statusBar.show();

	const all = await sessions();
	const mine = all.filter(belongsToWorkspace);
	const outside = all.length - mine.length;

	if (mine.length === 0) {
		statusBar.text = "$(circle-slash) Polaris";
		statusBar.tooltip = outside
			? `No Polaris session for this workspace.\n${outside} running from another window, left alone.\nClick to start one.`
			: "No Polaris session running for this workspace. Click to start one.";
		statusBar.backgroundColor = undefined;

		return;
	}

	const unhealthy = mine.some((session) => !session.healthy);
	const detached = mine.filter((session) => {
		const clients = clientsOf(session);

		return clients !== undefined && clients.length === 0;
	}).length;

	statusBar.text =
		`$(radio-tower) Polaris ${mine.map(portOf).join(", ")}` + (detached > 0 ? " $(debug-disconnect)" : "");
	statusBar.tooltip = mine
		.map((session) => {
			const studio = studioState(session);

			return `${projectName(session)} · ${vscode.workspace.asRelativePath(session.project)}\n${session.address}${
				session.healthy ? "" : " (unhealthy)"
			}${studio === undefined ? "" : `\n${studio}`}\npid ${session.pid}`;
		})
		.join("\n\n");

	statusBar.backgroundColor =
		unhealthy || detached > 0 ? new vscode.ThemeColor("statusBarItem.warningBackground") : undefined;
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
