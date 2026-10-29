local Polaris = script:FindFirstAncestor("Polaris")
local App = Polaris.App
local Components = App.Components
local Util = Components.Util

local Fusion = require(Polaris.Packages.Fusion)

local Theme = require(App.Theme)
local stripProps = require(Util.stripProps)

local New = Fusion.New
local Hydrate = Fusion.Hydrate
local Children = Fusion.Children

local RunService = game:GetService("RunService")

local COMPONENT_ONLY_PROPS = {
	"Duration",
}

local DOT_COUNT = 3
local DOT_SIZE = 7
local DOT_SPACING = 6

type Props = {
	Duration: number?,
	[any]: any,
}

-- Pulsing dots, drawn with plain frames: no image assets, so it matches the
-- widget theme and does not pull a bitmap in for a 12px animation.
return function(props: Props): Frame
	local duration = props.Duration or 1
	local dots = {}

	local children = {
		New "UIListLayout" {
			FillDirection = Enum.FillDirection.Horizontal,
			HorizontalAlignment = Enum.HorizontalAlignment.Center,
			VerticalAlignment = Enum.VerticalAlignment.Center,
			SortOrder = Enum.SortOrder.LayoutOrder,
			Padding = UDim.new(0, DOT_SPACING),
		},
	}

	for i = 1, DOT_COUNT do
		local dot = New "Frame" {
			Name = `Dot{i}`,
			LayoutOrder = i,
			Size = UDim2.fromOffset(DOT_SIZE, DOT_SIZE),
			BackgroundColor3 = Theme.Colors.Text,
			BackgroundTransparency = 0.7,
			BorderSizePixel = 0,
			[Children] = {
				New "UICorner" {
					CornerRadius = UDim.new(1, 0),
				},
			},
		}

		dots[i] = dot
		table.insert(children, dot)
	end

	local root = Hydrate(New "Frame" {
		Name = "Spinner",
		AnchorPoint = Vector2.new(0.5, 0.5),
		Size = UDim2.fromOffset(DOT_COUNT * DOT_SIZE + (DOT_COUNT - 1) * DOT_SPACING, DOT_SIZE),
		BackgroundTransparency = 1,
		BorderSizePixel = 0,
		[Children] = children,
	})(stripProps(props, COMPONENT_ONLY_PROPS))

	local function step()
		local now = os.clock() / duration

		for i, dot in dots do
			local phase = (now + (i - 1) / DOT_COUNT) % 1
			local wave = math.abs(math.sin(phase * math.pi))

			dot.BackgroundTransparency = 0.75 - 0.65 * wave
		end
	end

	step()

	local connection = RunService.Heartbeat:Connect(step)

	root.Destroying:Once(function()
		connection:Disconnect()
	end)

	return root
end
