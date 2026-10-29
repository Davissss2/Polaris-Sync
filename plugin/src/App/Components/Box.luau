local Polaris = script:FindFirstAncestor("Polaris")
local App = Polaris.App
local Components = App.Components
local Util = Components.Util

local Fusion = require(Polaris.Packages.Fusion)

local Theme = require(App.Theme)
local stripProps = require(Util.stripProps)

local Border = require(Components.Border)
local Container = require(Components.Container)

local Hydrate = Fusion.Hydrate
local Children = Fusion.Children

local COMPONENT_ONLY_PROPS = {
	"BackgroundColor",
	"BorderColor",
	"BorderGradient",
	"BackgroundTransparency",
	"BorderTransparency",
}

type Props = {
	BackgroundColor: Fusion.CanBeState<Color3>?,
	BorderColor: Fusion.CanBeState<Color3>?,
	BorderGradient: boolean?,
	BackgroundTransparency: Fusion.CanBeState<number>?,
	BorderTransparency: Fusion.CanBeState<number>?,
	[any]: any,
}

return function(props: Props): Frame
	return Hydrate(Container {
		Size = UDim2.fromOffset(120, Theme.CompSizeY.Large),
		BackgroundColor3 = props.BackgroundColor or Theme.Colors.Background,
		BackgroundTransparency = props.BackgroundTransparency or 0,
		[Children] = {
			Border {
				Color = props.BorderColor,
				Transparency = props.BorderTransparency,
				Gradient = props.BorderGradient,
			},
		},
	})(stripProps(props, COMPONENT_ONLY_PROPS))
end
