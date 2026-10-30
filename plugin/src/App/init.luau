local RunService = game:GetService("RunService")
local TweenService = game:GetService("TweenService")

local plugin = script:FindFirstAncestorWhichIsA("Plugin")

local Polaris = script:FindFirstAncestor("Polaris")
local Packages = Polaris.Packages
local Helpers = Polaris.Helpers

local Components = script.Components
local Widgets = script.Widgets
local Pages = script.Pages

local Fusion = require(Packages.Fusion)
local Signal = require(Packages.Signal)

local manifest = require(Polaris.manifest)
local Config = require(Polaris.Config)
local Log = require(Polaris.Log)
local Types = require(Polaris.Types)
local Util = require(Polaris.Util)
local Core = require(Polaris.Core)
local describeChanges = require(Helpers.describeChanges)

local Assets = require(script.Assets)
local Loader = require(script.Loader)
local Theme = require(script.Theme)

local Toolbar = require(Components.Plugin.Toolbar)
local ToolbarButton = require(Components.Plugin.ToolbarButton)
local Widget = require(Components.Plugin.Widget)
local Container = require(Components.Container)
local Padding = require(Components.Padding)
local Image = require(Components.Image)
local Text = require(Components.Text)
local List = require(Components.List)

local NotConnected = require(Pages.NotConnected)
local Connecting = require(Pages.Connecting)
local Connected = require(Pages.Connected)
local Unavailable = require(Pages.Unavailable)
local Prompt = require(Pages.Prompt)
local Error = require(Pages.Error)

local Settings = require(Widgets.Settings)
local Help = require(Widgets.Help)
local Diff = require(Widgets.Diff)

local New = Fusion.New
local Value = Fusion.Value
local Spring = Fusion.Spring
local OnEvent = Fusion.OnEvent
local OnChange = Fusion.OnChange
local Observer = Fusion.Observer
local Computed = Fusion.Computed
local Children = Fusion.Children
local cleanup = Fusion.cleanup
local peek = Fusion.peek

local RECONNECT_INTERVAL = 5

local App = {}
App.__index = App

function App.new()
	Loader.load()

	local self = setmetatable({}, App)

	self.core = nil
	self.lastUpdate = os.clock()
	self.onCanceled = Signal.new()

	self.host = Config:get("Host")
	self.port = Config:get("Port")

	self.lastSync = Value(os.time())
	self.lastSyncKind = Value("Unknown")

	-- Real sync state, driven by Core instead of assumed by the interface
	self.syncState = Value("Synced")
	self.syncError = Value("")

	self.rootSize = Value(Vector2.new(300, 200))
	self.icon = Value(Assets.Polaris.Logo)
	self.pages = Value({})

	self.settingsWidget = Value(nil)
	self.helpWidget = Value(nil)
	self.diffWidget = Value(nil)

	local isOpen = Value(false)

	local toolbarButton = ToolbarButton {
		Toolbar = Toolbar {
			Name = "Dervex Tools",
		},
		Name = "Polaris",
		ToolTip = "Open Polaris UI",
		Image = self.icon,
		[OnEvent "Click"] = function()
			isOpen:set(not peek(isOpen))
		end,
	}

	Widget {
		Name = "Polaris",
		MinimumSize = peek(self.rootSize),
		Enabled = isOpen,
		[OnChange "Enabled"] = function(isEnabled)
			isOpen:set(isEnabled)
		end,
		[OnChange "AbsoluteSize"] = function(size)
			self.rootSize:set(size)
		end,
		[Children] = self.pages,
	}

	plugin.Unloading:Connect(Observer(isOpen):onChange(function()
		toolbarButton:SetActive(peek(isOpen))
	end))

	toolbarButton:SetActive(peek(isOpen))

	if RunService:IsEdit() then
		self:home()

		if Config:get("AutoConnect") then
			self:connect()
		end
	else
		self:setPage(Unavailable())
	end

	return self
end

function App:setIcon(icon: ("Ok" | "Warn" | "Error")?)
	if icon == "Ok" then
		self.icon:set(Assets.Polaris.LogoOk)
	elseif icon == "Warn" then
		self.icon:set(Assets.Polaris.LogoWarn)
	elseif icon == "Error" then
		self.icon:set(Assets.Polaris.LogoError)
	else
		self.icon:set(Assets.Polaris.Logo)
	end
end

function App:setPage(page)
	local pages = peek(self.pages)

	for id, child in pages do
		if child._finished then
			page[id] = nil
			continue
		end

		task.spawn(function()
			child._destructor(child._value)
			child._finished = true
		end)
	end

	local size = Value(UDim2.fromScale(1, 1))

	-- Extremely hacky way to smoothly transition between pages
	page = Computed(function()
		return New "Frame" {
			AnchorPoint = Vector2.new(0.5, 0.5),
			Position = UDim2.fromScale(0.5, 0.5),
			Size = Spring(size, 10),
			BackgroundColor3 = Theme.Colors.Background,
			BorderSizePixel = 0,
			[Children] = {
				-- Polaris backdrop: a very subtle brand gradient instead of a flat fill
				New "UIGradient" {
					Rotation = 55,
					Color = Computed(function(use)
						if use(Theme.IsDark) then
							return ColorSequence.new({
								ColorSequenceKeypoint.new(0, Color3.fromRGB(12, 17, 33)),
								ColorSequenceKeypoint.new(1, Color3.fromRGB(23, 15, 41)),
							})
						end

						return ColorSequence.new(Color3.fromRGB(246, 249, 255))
					end),
				},
				List {
					VerticalAlignment = Enum.VerticalAlignment.Center,
					HorizontalAlignment = Enum.HorizontalAlignment.Center,
					Spacing = 8,
				},
				Padding {
					Padding = Theme.WidgetPadding,
				},
				Container {
					Size = UDim2.fromScale(1, 0),
					[Children] = {
						Image {
							Size = UDim2.fromOffset(155, 35),
							Image = Assets.Polaris.Banner,
						},
						Text {
							AnchorPoint = Vector2.new(1, 1),
							Position = UDim2.new(1, -3, 1, 0),
							Text = `v{manifest.package.version}`,
							Color = Theme.Colors.TextDimmed,
							TextSize = Theme.TextSize.Small,
						},
					},
				},
				Container {
					Size = UDim2.fromScale(1, 0),
					[Children] = page,
				},
			},
		}
	end, function(page)
		if not page then
			return
		end

		local TWEEN_INFO = TweenInfo.new(0.2, Enum.EasingStyle.Sine, Enum.EasingDirection.Out)
		TweenService:Create(page, TWEEN_INFO, { BackgroundTransparency = 1 }):Play()

		for _, descendant in page:GetDescendants() do
			local properties = {}

			if descendant:IsA("GuiObject") then
				if descendant:IsA("GuiButton") then
					descendant.Active = false
				end

				properties.BackgroundTransparency = 1

				if descendant:IsA("TextLabel") or descendant:IsA("TextButton") or descendant:IsA("TextBox") then
					properties.TextTransparency = 1
				elseif descendant:IsA("ImageLabel") or descendant:IsA("ImageButton") then
					properties.ImageTransparency = 1
				end
			end

			if descendant:IsA("UIStroke") then
				properties.Transparency = 1
			end

			if next(properties) then
				TweenService:Create(descendant, TWEEN_INFO, properties):Play()
			end
		end

		page.ZIndex = 2

		size:set(UDim2.fromScale(1.2, 1))

		task.wait(0.2)
		cleanup(page)
	end)

	pages[Util.generateGUID()] = page
	self.pages:set(pages)
end

function App:home()
	self.lastUpdate = os.clock()

	self:setPage(NotConnected {
		App = self,
		Host = self.host,
		Port = self.port,
	})

	self:setIcon()
end

function App:settings()
	local widget = peek(self.settingsWidget)

	if widget then
		widget:Destroy()
		widget = nil
	else
		widget = Settings {
			Closed = function()
				peek(self.settingsWidget):Destroy()
				self.settingsWidget:set(nil)
			end,
		}
	end

	self.settingsWidget:set(widget)
end

function App:help()
	local widget = peek(self.helpWidget)

	if widget then
		widget:Destroy()
		widget = nil
	else
		widget = Help {
			Closed = function()
				peek(self.helpWidget):Destroy()
				self.helpWidget:set(nil)
			end,
		}
	end

	self.helpWidget:set(widget)
end

function App:diff(changes: Types.Changes?, tree: any?)
	local widget = peek(self.diffWidget)

	if widget then
		widget:Destroy()
		widget = nil
	elseif changes and tree then
		widget = Diff {
			Changes = changes,
			Tree = tree,
			Closed = function()
				peek(self.diffWidget):Destroy()
				self.diffWidget:set(nil)
			end,
		}
	end

	self.diffWidget:set(widget)
end

-- A confirmation can also be answered from outside Studio with
-- `polaris consent`, which is what lets an agent unblock a session without a
-- click. The interface stays the primary way to answer
function App:awaitConsent(kind: string, message: string, signal: any, acceptValue: string): string
	local finished = false

	if self.core then
		self.core:announceConsent(kind, message)

		task.spawn(function()
			while not finished and self.core do
				task.wait(1)

				local ok, decision = pcall(function()
					return self.core and self.core:consentDecision():expect()
				end)

				if ok and decision and not finished then
					signal:Fire(if decision == "accept" then acceptValue else "Cancel")

					return
				end
			end
		end)
	end

	local choice = signal:Wait()
	finished = true

	-- Whatever answered it, the server should stop reporting the prompt
	if self.core then
		self.core:clearConsent()
	end

	return choice
end

function App:connect()
	local project = Value({})
	local canceled = false
	local loading = true

	self.core = Core.new(self.host, self.port)

	self.syncState:set("Syncing")
	self.syncError:set("")

	task.spawn(function()
		task.wait(0.2)

		if not self:isConnected() and loading then
			self:setPage(Connecting {
				App = self,
			})
		end
	end)

	self.onCanceled:Once(function()
		canceled = true
	end)

	self.core:onPrompt(function(message, changes, tree)
		loading = false

		local signal = Signal.new()
		local result = false

		self:setIcon("Warn")

		-- Prompt user to accept or reject incoming changes
		if changes then
			local options = { "Accept", "Diff", "Cancel" }
			message = describeChanges(changes)

			self:setPage(Prompt {
				App = self,
				Message = message,
				Options = options,
				Signal = signal,
			})

			local choice = self:awaitConsent("changes", message, signal, "Accept")

			while choice == "Diff" do
				self:diff(changes, tree)
				choice = signal:Wait()
			end

			self:diff()

			result = choice == "Accept"
		-- Prompt user whether to continue when server ID differs
		else
			local options = { "Continue", "Cancel" }

			self:setPage(Prompt {
				App = self,
				Message = message,
				Options = options,
				Signal = signal,
			})

			result = self:awaitConsent("project", message, signal, "Continue") == "Continue"
		end

		if self.core.status == Core.Status.Connected then
			self:setPage(Connected {
				App = self,
				Project = project,
			})

			self:setIcon("Ok")
		end

		return result
	end)

	self.core:onReady(function(projectDetails)
		loading = false
		project:set(projectDetails)

		self.syncState:set("Synced")
		self.syncError:set("")

		self.lastSync:set(os.time())

		self.lastSyncKind:set(Config:get("InitialSyncPriority"))

		self:setPage(Connected {
			App = self,
			Project = project,
		})

		self:setIcon("Ok")
	end)

	self.core:onSync(function(kind, data)
		self.lastSync:set(os.time())
		self.lastSyncKind:set(
			(kind == "SyncChanges" or kind == "SyncDetails") and "Server"
				or kind == "SyncbackChanges" and "Client"
				or "None"
		)

		if kind == "SyncDetails" then
			project:set(data)
		end
	end)

	self.core:onProgress(function(state, detail)
		self.syncState:set(state)

		if state == "Error" then
			self.syncError:set(
				if type(detail) == "string"
					then detail
					elseif detail and detail.message then detail.message
					else tostring(detail)
			)

			self:setIcon("Error")
		elseif state == "Syncing" then
			self:setIcon("Warn")
		else
			self.syncError:set("")

			self:setIcon("Ok")
		end
	end)

	self.core:run():catch(function(err)
		loading = false

		if Core.wasExitGraceful(err) or canceled then
			self:disconnect()
			self:home()
			return
		end

		self:setPage(Error {
			App = self,
			Message = err.message or tostring(err),
		})

		self.syncState:set("Error")
		self.syncError:set(err.message or tostring(err))

		self:setIcon("Error")

		self:disconnect()

		if Config:get("AutoReconnect") then
			task.wait(RECONNECT_INTERVAL)

			-- The user may have reconnected or canceled while this thread waited,
			-- and the old timestamp check made the retry easy to skip for good
			if self.core == nil and not canceled then
				Log.info("Reconnecting to the Polaris server..")

				self:connect()
			end
		end
	end)
end

function App:disconnect(cancel: boolean?)
	self.lastUpdate = os.clock()

	if self.core then
		if cancel then
			self.onCanceled:Fire()
		end

		self.core:stop()
		self.core = nil
	end
end

function App:isConnected()
	return self.core and self.core.status == Core.Status.Connected
end

function App:setHost(host: string)
	self.host = host
end

function App:setPort(port: number)
	self.port = port
end

return App
