local ContentProvider = game:GetService("ContentProvider")

local Polaris = script:FindFirstAncestor("Polaris")

local Log = require(Polaris.Log)

local Assets = require(script.Parent.Assets)

local Loader = {}

-- Only the strings inside an asset group are asset ids. The file also carries
-- notes for whoever edits it, and preloading those as images made Studio fail
-- with an "Invalid image or texture" error on every load
function Loader.assets(): { string }
	local assets = {}

	for _, group in Assets do
		if type(group) ~= "table" then
			continue
		end

		for _, value in group do
			if type(value) == "string" then
				table.insert(assets, value)
			end
		end
	end

	return assets
end

function Loader.load()
	local assets = Loader.assets()

	Log.trace(`Loading {#assets} assets..`)

	task.spawn(function()
		ContentProvider:PreloadAsync(assets)
	end)
end

return Loader
