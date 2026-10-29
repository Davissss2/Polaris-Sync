local Polaris = script:FindFirstAncestor("Polaris")
local App = Polaris.App
local Widgets = App.Widgets
local Components = App.Components

local Fusion = require(Polaris.Packages.Fusion)

local Types = require(Polaris.Types)
local Theme = require(App.Theme)

local Project = require(Widgets.Project)
local TextButton = require(Components.TextButton)
local IconButton = require(Components.IconButton)
local Container = require(Components.Container)
local Padding = require(Components.Padding)
local Text = require(Components.Text)
local List = require(Components.List)
local Corner = require(Components.Corner)
local Box = require(Components.Box)

local Value = Fusion.Value
local New = Fusion.New
local Observer = Fusion.Observer
local Computed = Fusion.Computed
local Children = Fusion.Children
local Cleanup = Fusion.Cleanup
local peek = Fusion.peek

type Props = {
	App: { [string]: any },
	Project: Fusion.Value<Types.Project>,
}

-- Colours and labels for the real sync state reported by Core
local function statusColor(use, state: string): Color3
	if state == "Error" then
		return use(Theme.Colors.Danger)
	elseif state == "Syncing" then
		return use(Theme.Colors.BrandCyan)
	end

	return use(Theme.Colors.Success)
end

local function statusLabel(state: string): string
	if state == "Error" then
		return "Error"
	elseif state == "Syncing" then
		return "Syncing"
	end

	return "Synced"
end

return function(props: Props): { Instance }
	local syncedText = Value("just now")
	local projectWidget = Value(nil)

	local function updateSyncedText()
		return task.spawn(function()
			while true do
				local elapsed = os.time() - peek(props.App.lastSync)
				local interval = 1
				local text

				if elapsed < 1 then
					text = "just now"
				elseif elapsed < 60 then
					text = math.floor(elapsed) .. "s ago"
				elseif elapsed < 3600 then
					interval = 60
					text = elapsed // interval .. "m ago"
				elseif elapsed < 86400 then
					interval = 3600
					text = elapsed // interval .. "h ago"
				else
					interval = 86400
					text = elapsed // interval .. "d ago"
				end

				syncedText:set(text)
				task.wait(interval)
			end
		end)
	end

	local thread = updateSyncedText()
	local disconnect = Observer(props.App.lastSync):onChange(function()
		task.cancel(thread)
		thread = updateSyncedText()
	end)

	return {
		List {},
		Box {
			Size = UDim2.new(1, 0, 0, Theme.CompSizeY.Large * 2),
			BackgroundColor = Theme.Colors.Surface,
			BorderGradient = true,
			BorderTransparency = 0.45,
			[Children] = {
				Padding {},

				-- Project name
				Text {
					Size = UDim2.new(0.55, 0, 0.55, 0),
					Text = Computed(function(use)
						return use(props.Project).name
					end),
					Font = Theme.Fonts.Bold,
					Scaled = true,
				},

				-- Server address
				Text {
					Position = UDim2.fromScale(0, 0.58),
					Size = UDim2.new(0.55, 0, 0.4, 0),
					Text = `{props.App.host}:{props.App.port}`,
					TextSize = Theme.TextSize.Small,
					Font = Theme.Fonts.Mono,
					Color = Theme.Colors.TextDimmed,
					Scaled = true,
				},

				-- Status pill, driven by the state Core reports
				Box {
					AnchorPoint = Vector2.new(1, 0),
					Position = UDim2.new(1, 0, 0, 0),
					Size = UDim2.fromOffset(96, 26),
					BackgroundColor = Theme.Colors.ActiveBackground,
					BorderColor = Computed(function(use)
						return statusColor(use, use(props.App.syncState))
					end),
					BorderTransparency = 0.6,
					[Children] = {
						Corner { Radius = 13 },
						New "Frame" {
							AnchorPoint = Vector2.new(0, 0.5),
							Position = UDim2.new(0, 12, 0.5, 0),
							Size = UDim2.fromOffset(8, 8),
							BackgroundColor3 = Computed(function(use)
								return statusColor(use, use(props.App.syncState))
							end),
							BorderSizePixel = 0,
							[Children] = { Corner { Radius = 4 } },
						},
						Text {
							AnchorPoint = Vector2.new(0, 0.5),
							Position = UDim2.new(0, 27, 0.5, 0),
							Size = UDim2.fromOffset(62, 20),
							Text = Computed(function(use)
								return statusLabel(use(props.App.syncState))
							end),
							TextSize = Theme.TextSize.Small,
							Font = Theme.Fonts.Bold,
						},
					},
				},

				-- Last sync + direction
				Text {
					AnchorPoint = Vector2.new(1, 0),
					Position = UDim2.new(1, 0, 0, 32),
					Size = UDim2.new(0.4, 0, 0.4, 0),
					Text = Computed(function(use)
						local lastSyncKind = use(props.App.lastSyncKind)
						local action = ""

						if lastSyncKind == "Server" then
							action = "↓ "
						elseif lastSyncKind == "Client" then
							action = "↑ "
						end

						return action .. use(syncedText)
					end),
					Color = Theme.Colors.TextDimmed,
					TextSize = Theme.TextSize.Small,
					TextXAlignment = Enum.TextXAlignment.Right,
					Scaled = true,
				},
			},
		},
		-- Error strip: only laid out when the last operation failed
		Box {
			Size = UDim2.new(1, 0, 0, 46),
			BackgroundColor = Theme.Colors.Surface,
			BorderColor = Theme.Colors.Danger,
			BorderTransparency = 0.35,
			Visible = Computed(function(use)
				return use(props.App.syncState) == "Error"
			end),
			[Children] = {
				Padding {},
				Text {
					Size = UDim2.fromScale(1, 1),
					Text = Computed(function(use)
						local message = use(props.App.syncError)

						return if message == "" then "The last sync failed" else message
					end),
					TextSize = Theme.TextSize.Small,
					Color = Theme.Colors.Danger,
					AutomaticSize = Enum.AutomaticSize.None,
					TextWrapped = true,
					TextYAlignment = Enum.TextYAlignment.Center,
				},
			},
		},
		Container {
			Size = UDim2.fromScale(1, 0),
			[Children] = {
				List {
					FillDirection = Enum.FillDirection.Horizontal,
					HorizontalAlignment = Enum.HorizontalAlignment.Right,
				},
				TextButton {
					Text = "Disconnect",
					LayoutOrder = 2,
					Solid = true,
					Activated = function()
						props.App:disconnect()
						props.App:home()
					end,
				},
				IconButton {
					Glyph = "Settings",
					BackgroundActive = Computed(function(use)
						return use(props.App.settingsWidget) ~= nil
					end),
					Activated = function()
						props.App:settings()
					end,
					LayoutOrder = 1,
				},
				IconButton {
					Glyph = "Info",
					BackgroundActive = Computed(function(use)
						return use(projectWidget) ~= nil
					end),
					Activated = function()
						local widget = peek(projectWidget)

						if widget then
							widget:Destroy()
							widget = nil
						else
							widget = Project {
								App = props.App,
								Project = props.Project,

								Closed = function()
									peek(projectWidget):Destroy()
									projectWidget:set(nil)
								end,
							}
						end

						projectWidget:set(widget)
					end,
					[Cleanup] = function()
						local widget = peek(projectWidget)

						if widget then
							widget:Destroy()
							widget = nil
						end

						disconnect()
						task.cancel(thread)
					end,
				},
			},
		},
	}
end
