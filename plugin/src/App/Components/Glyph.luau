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

-- Polaris draws its own icons on a 16x16 canvas from frames and strokes, so the
-- interface carries no inherited bitmaps and stays sharp at any scale.
local BASE = 16

local COMPONENT_ONLY_PROPS = {
	"Glyph",
	"Color",
	"Transparency",
	"Thickness",
	"Scale",
}

type Props = {
	Glyph: string?,
	Color: Fusion.CanBeState<Color3>?,
	Transparency: Fusion.CanBeState<number>?,
	Thickness: number?,
	Scale: number?,
	[any]: any,
}

return function(props: Props): Frame
	local color = props.Color or Theme.Colors.Text
	local transparency = props.Transparency or 0
	local thickness = props.Thickness or 1.6

	local function bar(width: number, height: number, x: number, y: number, rotation: number?): Frame
		return New "Frame" {
			AnchorPoint = Vector2.new(0.5, 0.5),
			Position = UDim2.fromOffset(x, y),
			Size = UDim2.fromOffset(width, height),
			Rotation = rotation or 0,
			BackgroundColor3 = color,
			BackgroundTransparency = transparency,
			BorderSizePixel = 0,
			[Children] = {
				New "UICorner" { CornerRadius = UDim.new(1, 0) },
			},
		}
	end

	local function dot(size: number, x: number, y: number): Frame
		return New "Frame" {
			AnchorPoint = Vector2.new(0.5, 0.5),
			Position = UDim2.fromOffset(x, y),
			Size = UDim2.fromOffset(size, size),
			BackgroundColor3 = color,
			BackgroundTransparency = transparency,
			BorderSizePixel = 0,
			[Children] = {
				New "UICorner" { CornerRadius = UDim.new(1, 0) },
			},
		}
	end

	local function ring(diameter: number): Frame
		return New "Frame" {
			AnchorPoint = Vector2.new(0.5, 0.5),
			Position = UDim2.fromOffset(BASE / 2, BASE / 2),
			Size = UDim2.fromOffset(diameter, diameter),
			BackgroundTransparency = 1,
			[Children] = {
				New "UICorner" { CornerRadius = UDim.new(1, 0) },
				New "UIStroke" {
					Thickness = thickness,
					Color = color,
					Transparency = transparency,
					ApplyStrokeMode = Enum.ApplyStrokeMode.Border,
				},
			},
		}
	end

	local function glyphLabel(text: string, textSize: number): TextLabel
		return New "TextLabel" {
			AnchorPoint = Vector2.new(0.5, 0.5),
			Position = UDim2.fromOffset(BASE / 2, BASE / 2),
			Size = UDim2.fromOffset(BASE, textSize * 1.5),
			BackgroundTransparency = 1,
			BorderSizePixel = 0,
			Text = text,
			TextSize = textSize,
			FontFace = Theme.Fonts.Bold,
			TextColor3 = color,
			TextTransparency = transparency,
		}
	end

	local builders = {}

	-- Gear: a ring with radial teeth and a hub
	function builders.Settings()
		local children = { ring(8.5) }

		for i = 1, 6 do
			local angle = (i - 1) * math.pi / 3

			table.insert(
				children,
				bar(3, 1.6, BASE / 2 + math.cos(angle) * 5.4, BASE / 2 + math.sin(angle) * 5.4, math.deg(angle))
			)
		end

		table.insert(children, dot(2.6, BASE / 2, BASE / 2))

		return children
	end

	-- Info: ring with a lower case i
	function builders.Info()
		return {
			ring(12),
			dot(2.2, BASE / 2, 5),
			bar(2.2, 5, BASE / 2, 10),
		}
	end

	-- Help: ring with a question mark (plain ASCII, no font gamble)
	function builders.Help()
		return {
			ring(12.5),
			glyphLabel("?", 9),
		}
	end

	-- Check: two strokes
	function builders.Check()
		return {
			bar(4.4, 2.2, 6.2, 9.6, 45),
			bar(8.6, 2.2, 9.8, 7.2, -50),
		}
	end

	-- Chevrons: two strokes forming a V
	function builders.ChevronDown()
		return {
			bar(5.4, 2, 5.8, 8.6, 38),
			bar(5.4, 2, 10.2, 8.6, -38),
		}
	end

	function builders.ChevronRight()
		return builders.ChevronDown()
	end

	-- External: arrow leaving towards the top right
	function builders.External()
		return {
			bar(9.4, 2, 7.4, 8.6, -45),
			bar(4.6, 2, 10.6, 3.4, 0),
			bar(2, 4.6, 12.6, 5.4, 0),
		}
	end

	local children = {
		New "UIScale" { Scale = props.Scale or 1 },
	}

	if props.Glyph and builders[props.Glyph] then
		for _, child in builders[props.Glyph]() do
			table.insert(children, child)
		end
	end

	return Hydrate(New "Frame" {
		Name = "Glyph",
		AnchorPoint = Vector2.new(0.5, 0.5),
		Size = UDim2.fromOffset(BASE, BASE),
		BackgroundTransparency = 1,
		BorderSizePixel = 0,
		Rotation = if props.Glyph == "ChevronRight" then 90 else 0,
		[Children] = children,
	})(stripProps(props, COMPONENT_ONLY_PROPS))
end
