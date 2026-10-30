local ScriptEditorService = game:GetService("ScriptEditorService")

local Polaris = script:FindFirstAncestor("Polaris")

local Promise = require(Polaris.Packages.Promise)

local Client = require(Polaris.Client)
local Config = require(Polaris.Config)
local Log = require(Polaris.Log)
local Util = require(Polaris.Util)
local Types = require(Polaris.Types)
local Watcher = require(Polaris.Watcher)
local Executor = require(Polaris.Executor)
local SemVer = require(Polaris.SemVer)
local manifest = require(Polaris.manifest)
local equals = require(Polaris.Helpers.equals)

local Processor = require(script.Processor)
local Changes = require(script.Changes)
local Tree = require(script.Tree)
local Error = require(script.Error)

local SYNCBACK_RATE = 0.5
local SYNCBACK_DEBOUNCE = 0.1

local Core = {
	Status = {
		Disconnected = 0,
		Connecting = 1,
		Connected = 2,
		Disconnecting = 3,
	},
}
Core.__index = Core

function Core.new(host: string?, port: string?)
	local self = setmetatable({}, Core)

	self.project = nil
	self.rootInstances = {}
	self.connections = {}
	self.status = 0
	self.lastSync = os.clock()

	self.tree = Tree.new()
	self.client = Client.new(host or Config:get("Host"), port or Config:get("Port"), Config:get("Https"))
	self.processor = Processor.new(self.tree, self.client)
	self.watcher = Watcher.new()
	self.executor = Executor.new()

	self.__prompt = function(_message: string, _changes: Types.Changes?, _tree: any): boolean
		return true
	end
	self.__ready = function(_project: Types.Project) end
	self.__sync = function(_kind: Types.MessageKind, _data: any) end
	self.__progress = function(_state: "Synced" | "Syncing" | "Error", _detail: any) end

	self:__handleOpenInEditor(Config:get("OpenInEditor"))

	-- Watch for `OpenInEditor setting
	table.insert(
		self.connections,
		Config:onChanged("OpenInEditor", function(enabled)
			self:__handleOpenInEditor(enabled)
		end)
	)

	-- Watch for `TwoWaySync` setting
	table.insert(
		self.connections,
		Config:onChanged("TwoWaySync", function(enabled)
			if enabled then
				self.watcher:start(self.rootInstances)
			else
				self.watcher:stop()
			end
		end)
	)

	return self
end

function Core:run(): Promise.Promise
	return Promise.new(function(_, reject)
		self.status = Core.Status.Connecting

		local initialSyncPriority = Config:get("InitialSyncPriority")

		Log.trace("Fetching server details..")

		local project = self.client:fetchDetails():expect()
		self.project = project

		self:__verifyProject(project, true):expect()

		Log.trace("Subscribing to the server queue..")

		self.client:subscribe():expect()

		Log.trace("Getting initial snapshot..")

		local snapshot = self.client:getSnapshot():expect()

		Log.trace("Initializing processor..")

		local changes = self.processor:init(snapshot, initialSyncPriority)

		if self.status ~= Core.Status.Connecting then
			return reject(Error.new(Error.Terminated))
		end

		for i, id in project.rootRefs do
			self.rootInstances[i] = self.tree:getInstance(id)
		end

		if initialSyncPriority ~= "None" then
			Log.trace("Processing initial snapshot..")

			if initialSyncPriority == "Server" then
				self:__verifyChanges(changes, true):expect()
				self.processor.write:applyChanges(changes, true)
			elseif changes:total() > 0 then
				local reversed = self.processor:reverseChanges(changes)
				self.client:write(reversed):expect()
			end
		end

		if Config:get("TwoWaySync") then
			self.watcher:start(self.rootInstances)
		end

		self.status = Core.Status.Connected
		self.__ready(project)

		Log.trace("Starting sync loops..")

		self:__startSyncbackLoop():catch(function(err)
			return reject(err)
		end)

		return self:__startSyncLoop():expect()
	end)
end

function Core:stop()
	Log.trace("Stopping Polaris core..")

	self.status = Core.Status.Disconnecting

	self.watcher:stop()

	if self.client.isSubscribed then
		task.spawn(function()
			-- A prompt that nobody answers would otherwise stay listed after
			-- the plugin is gone
			self:clearConsent()

			self.client:unsubscribe():catch(function(err)
				Log.debug("Failed to unsubscribe from the server", err)
			end)
		end)
	end

	Util.clean(self.connections)
end

function Core:onPrompt(callback: (message: string, changes: Types.Changes?, tree: any) -> boolean)
	self.__prompt = function(message, changes)
		if self.status == Core.Status.Disconnecting then
			return false
		end

		return callback(message, changes, self.tree)
	end
end

function Core:onReady(callback: (project: Types.Project) -> ())
	self.__ready = function(project)
		if self.status == Core.Status.Disconnecting then
			return
		end

		return callback(project)
	end
end

function Core:onSync(callback: (kind: Types.MessageKind, data: any) -> ())
	self.__sync = function(kind, data)
		if self.status == Core.Status.Disconnecting then
			return
		end

		return callback(kind, data)
	end
end

-- Reports the state of an in-flight sync operation, so the interface can show
-- what is actually happening instead of assuming everything went fine
function Core:onProgress(callback: (state: "Synced" | "Syncing" | "Error", detail: any) -> ())
	self.__progress = function(state, detail)
		if self.status == Core.Status.Disconnecting then
			return
		end

		return callback(state, detail)
	end
end

-- Tells the server which confirmation is on screen, so it stops being
-- invisible outside Studio. Older servers answer 404 and this is a no-op
function Core:announceConsent(kind: string, message: string)
	return self.client:announceConsent(kind, message):catch(function(err)
		Log.trace("Failed to announce the confirmation prompt", err)
	end)
end

function Core:consentDecision(): Promise.TypedPromise<string?>
	return self.client:readConsent()
end

function Core:clearConsent()
	return self.client:clearConsent():catch(function(err)
		Log.trace("Failed to clear the confirmation prompt", err)
	end)
end

function Core.wasExitGraceful(err: Error.Error)
	return err == Error.GameId
		or err == Error.PlaceIds
		or err == Error.TooManyChanges
		or err == Error.MassDeletion
		or err == Error.UnboundProject
		or err == Error.Terminated
end

-- Internal functions

function Core:__verifyProject(project: Types.Project, initial: boolean?): Promise.Promise
	-- A project that declares neither a game nor its places is accepted by any
	-- place, and applying it merges into whatever is open. That has to be a
	-- conscious choice, so it is always confirmed, even with prompts off
	if initial and not project.gameId and #project.placeIds == 0 then
		local err = Error.new(Error.UnboundProject, project.name)

		if not self.__prompt(err.message) then
			return Promise.reject(err)
		end
	end

	if not self:__shouldPrompt(initial) then
		return Promise.resolve()
	end

	if project.gameId and project.gameId ~= game.GameId then
		local err = Error.new(Error.GameId, game.GameId, project.gameId)

		if not self.__prompt(err.message) then
			return Promise.reject(err)
		end
	end

	if #project.placeIds > 0 and not table.find(project.placeIds, game.PlaceId) then
		local err = Error.new(Error.PlaceIds, game.PlaceId, project.placeIds)

		if not self.__prompt(err.message) then
			return Promise.reject(err)
		end
	end

	if not SemVer.parse(project.version):isCompatible(SemVer.parse(manifest.package.version)) then
		local err = Error.new(Error.Version, manifest.package.version, project.version)

		return Promise.reject(err)
	end

	return Promise.resolve()
end

function Core:__verifyChanges(changes: Types.Changes, initial: boolean?): Promise.Promise
	local removals = #changes.removals

	-- Deleting instances cannot be undone from the disk, so a mass removal is
	-- always confirmed, even when prompts are turned off
	if removals > Config:get("RemovalsThreshold") then
		local err = Error.new(Error.MassDeletion, removals)

		if not self.__prompt(err.message, changes) then
			return Promise.reject(err)
		end

		return Promise.resolve()
	end

	if not self:__shouldPrompt(initial) then
		return Promise.resolve()
	end

	if Changes.Total(changes) > Config:get("ChangesThreshold") then
		local err = Error.new(Error.TooManyChanges, #changes.additions, #changes.updates, #changes.removals)

		if not self.__prompt(err.message, changes) then
			return Promise.reject(err)
		end
	end

	return Promise.resolve()
end

function Core:__startSyncLoop()
	return Promise.new(function(resolve, reject)
		while self.status == Core.Status.Connected do
			local message = self.client:read():expect() :: Types.Message?

			if not message then
				continue
			end

			local kind = next(message) :: Types.MessageKind
			local data = message[kind] :: any

			Log.trace("Received message:", kind)

			if kind == "SyncChanges" then
				self:__verifyChanges(data):expect()

				self.__progress("Syncing", data)

				local ok, result = pcall(self.processor.write.applyChanges, self.processor.write, data)

				if not ok then
					self.__progress("Error", result)

					return reject(Error.new(Error.ApplyFailed, tostring(result)))
				end

				self.lastSync = os.clock()

				self.__progress("Synced", data)
				self.__sync(kind, data)
			elseif kind == "SyncDetails" then
				self:__verifyProject(data):expect()

				if not equals(self.project.rootRefs, data.rootRefs) then
					self.rootInstances = {}

					for i, id in data.rootRefs do
						self.rootInstances[i] = self.tree:getInstance(id)
					end

					self.watcher:restart(self.rootInstances)
				end

				self.project = data

				self.__sync(kind, data)
			elseif kind == "ExecuteCode" then
				self.executor:execute(data.code)
			elseif kind == "Disconnect" then
				reject(Error.new(Error.Disconnected, data.message))
			else
				local err = Error.new(Error.UnknownEvent, kind, data)
				Log.warn(err)
			end
		end

		resolve()
	end)
end

function Core:__startSyncbackLoop()
	local aggregateChanges = Changes.new()

	return Promise.new(function(resolve)
		task.spawn(function()
			while self.status == Core.Status.Connected do
				task.wait(SYNCBACK_RATE)

				if aggregateChanges:isEmpty() then
					continue
				end

				local changes = aggregateChanges
				aggregateChanges = Changes.new()

				-- Removing something in Studio deletes its files from the disk.
				-- The count is instances, not files: deleting one folder takes
				-- every file under it, so any removal is confirmed here
				if #changes.removals > 0 then
					local err = Error.new(Error.MassSyncback)

					if not self.__prompt(err.message) then
						Log.warn("Discarded a sync back with", #changes.removals, "removals")

						continue
					end
				end

				self.__progress("Syncing", changes)

				self.client
					:write(changes)
					:andThen(function()
						self.__progress("Synced", changes)
					end)
					:catch(function(err)
						self.__progress("Error", err)

						Log.warn("Failed to write changes to the server:", err)
					end)

				self.__sync("SyncbackChanges", changes)
			end
		end)

		while self.status == Core.Status.Connected do
			local event = self.watcher:listen() :: Types.WatcherEvent

			if os.clock() - self.lastSync <= SYNCBACK_DEBOUNCE then
				continue
			end

			local kind = event.kind
			local changes = Changes.new()

			Log.trace("Received event:", kind)

			if kind == "Add" then
				local snapshot = self.processor.read:onAdd(event.instance)

				if snapshot then
					changes:add(snapshot)
				end
			elseif kind == "Change" then
				local snapshot = self.processor.read:onChange(event.instance, event.property)

				if snapshot then
					changes:update(snapshot)
				end
			elseif kind == "Remove" then
				local id = self.processor.read:onRemove(event.instance)

				if id then
					changes:remove(id)
				end
			end

			aggregateChanges:join(changes, true)
		end

		resolve()
	end)
end

function Core:__handleOpenInEditor(enabled: boolean)
	if not enabled then
		local connection = self.connections["openInEditor"]

		if connection then
			connection:Disconnect()
			self.connections["openInEditor"] = nil
		end

		return
	end

	self.connections["openInEditor"] = ScriptEditorService.TextDocumentDidOpen:Connect(function(document)
		if self.status ~= Core.Status.Connected then
			return
		end

		if document:IsCommandBar() then
			Log.trace("Document is a command bar, ignoring")
			return
		end

		local id = self.tree:getId(document:GetScript())

		if not id then
			Log.trace("Document is not synced by Polaris, ignoring")
			return
		end

		local line = document:GetSelectionStart()

		self.client
			:open(id, line)
			:andThen(function()
				document:CloseAsync()
			end)
			:catch(function(err)
				Log.debug("Failed to open document in editor:", err)
			end)
	end)
end

function Core:__cleanConnection(id: string)
	local connection = self.connections[id]

	if connection then
		connection:Disconnect()
		self.connections[id] = nil
	end
end

function Core:__shouldPrompt(initial: boolean?): boolean
	local mode = Config:get("DisplayPrompts")

	if mode == "Always" then
		return true
	elseif mode == "Initial" and initial then
		return true
	else
		return false
	end
end

return Core
