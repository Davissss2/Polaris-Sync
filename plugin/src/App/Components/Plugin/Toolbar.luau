local plugin = script:FindFirstAncestorWhichIsA("Plugin")

local Polaris = script:FindFirstAncestor("Polaris")
local App = Polaris.App
local Components = App.Components
local Util = Components.Util

local Fusion = require(Polaris.Packages.Fusion)

local stripProps = require(Util.stripProps)

local Hydrate = Fusion.Hydrate

local COMPONENT_ONLY_PROPS = {
	"Name",
}

type Props = {
	Name: string,
	[any]: any,
}

return function(props: Props): PluginToolbar
	return Hydrate(plugin:CreateToolbar(props.Name))(stripProps(props, COMPONENT_ONLY_PROPS))
end
