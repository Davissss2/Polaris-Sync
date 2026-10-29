local Polaris = script:FindFirstAncestor("Polaris")
local App = Polaris.App
local Components = App.Components

local Fusion = require(Polaris.Packages.Fusion)

local Theme = require(App.Theme)

local Corner = require(Components.Corner)

local New = Fusion.New
local Children = Fusion.Children

type Props = {
	Color: Fusion.CanBeState<Color3>?,
	Transparency: Fusion.CanBeState<number>?,
	Thickness: Fusion.CanBeState<number>?,
	CornerRadius: Fusion.CanBeState<UDim>?,
	Gradient: boolean?,
}

-- Literal colours: Theme exposes Fusion values, which ColorSequence cannot take
local BRAND_GRADIENT = ColorSequence.new({
	ColorSequenceKeypoint.new(0, Color3.fromRGB(80, 200, 245)),
	ColorSequenceKeypoint.new(0.5, Color3.fromRGB(59, 103, 238)),
	ColorSequenceKeypoint.new(1, Color3.fromRGB(139, 92, 246)),
})

return function(props: Props): { Instance }
	return {
		Corner {},
		New "UIStroke" {
			ApplyStrokeMode = Enum.ApplyStrokeMode.Border,
			Color = props.Color or Theme.Colors.Border,
			Thickness = props.Thickness or Theme.BorderThickness,
			Transparency = props.Transparency or 0,
			[Children] = if props.Gradient
				then New "UIGradient" {
					Color = BRAND_GRADIENT,
					Rotation = 45,
				}
				else nil,
		},
	}
end
