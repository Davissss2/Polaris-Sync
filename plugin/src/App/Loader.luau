local ContentProvider = game:GetService("ContentProvider")

local Polaris = script:FindFirstAncestor("Polaris")

local Util = require(Polaris.Util)
local Log = require(Polaris.Log)

local Assets = require(script.Parent.Assets)

local Loader = {}

function Loader.assets(): { string }
	return Util.dictionaryToArray(Assets)
end

function Loader.load()
	local assets = Loader.assets()

	Log.trace(`Loading {#assets} assets..`)

	task.spawn(function()
		ContentProvider:PreloadAsync(assets)
	end)
end

return Loader
