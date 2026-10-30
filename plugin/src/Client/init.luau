local Polaris = script:FindFirstAncestor("Polaris")

local Promise = require(Polaris.Packages.Promise)

local Types = require(Polaris.Types)
local Place = require(Polaris.Place)
local Log = require(Polaris.Log)

local Http = require(script.Http)
local Error = require(script.Error)

local function generateId(): number
	return math.floor(math.random() * 10 ^ 9)
end

local Client = {}
Client.__index = Client

function Client.new(host: string, port: number, https: boolean)
	local self = setmetatable({}, Client)

	self.isSubscribed = false
	self.clientId = generateId()
	self.placeName = Place.Name
	self.host = host
	self.port = port
	self.https = https

	return self
end

function Client:getUrl(endpoint: string?): string
	local protocol = self.https and "https" or "http"
	return `{protocol}://{self.host}:{self.port}/{endpoint or ""}`
end

function Client:fetchDetails(): Promise.TypedPromise<Types.Project>
	Log.trace("Client request: details")

	local url = self:getUrl("details")

	return Http.get(url):andThen(function(response)
		return response:decode()
	end)
end

function Client:subscribe(): Promise.Promise
	Log.trace("Client request: subscribe")

	local url = self:getUrl("subscribe")

	return Http.post(url, {
		clientId = self.clientId,
		name = self.placeName,
		-- The server refuses a project bound to another place, so the mistake
		-- cannot be applied even if someone accepts the prompt by hand
		placeId = game.PlaceId,
		gameId = game.GameId,
	})
		:andThen(function()
			self.isSubscribed = true
		end)
		:catch(function(err)
			if err == Error.Unknown then
				return Promise.reject(Error.new(Error.AlreadySubscribed, self.clientId))
			else
				return Promise.reject(err)
			end
		end)
end

function Client:unsubscribe(): Promise.Promise
	Log.trace("Client request: unsubscribe")

	local url = self:getUrl("unsubscribe")

	return Http.post(url, {
		clientId = self.clientId,
	})
		:andThen(function()
			self.isSubscribed = false
		end)
		:catch(function(err)
			if err == Error.Unknown then
				return Promise.reject(Error.new(Error.NotSubscribed, self.clientId))
			else
				return Promise.reject(err)
			end
		end)
end

function Client:getSnapshot(instance: Types.Ref?): Promise.TypedPromise<Types.AddedSnapshot>
	Log.trace("Client request: snapshot")

	local url = self:getUrl("snapshot")

	return Http.post(url, {
		instance = instance or buffer.create(16),
	}):andThen(function(response)
		return response:decode()
	end)
end

function Client:read(): Promise.TypedPromise<Types.Message>
	Log.trace("Client request: read")

	local url = self:getUrl("read")

	return Promise.new(function(resolve, reject)
		local function attempt()
			Http.post(url, {
				clientId = self.clientId,
			})
				:andThen(function(response)
					resolve(response:decode())
				end)
				:catch(function(err)
					-- The server answers an empty message once its long poll expires,
					-- so a timeout only means that nothing changed yet. Deferring the
					-- retry keeps the stack flat during long sessions
					if err == Error.Timedout then
						task.defer(attempt)
					else
						reject(err)
					end
				end)
		end

		attempt()
	end)
end

function Client:write(changes: Types.Changes): Promise.Promise
	Log.trace("Client request: write")

	local url = self:getUrl("write")

	return Http.post(url, {
		clientId = self.clientId,
		changes = changes,
	})
end

function Client:announceConsent(kind: string, message: string): Promise.Promise
	Log.trace("Client request: consent")

	local url = self:getUrl("consent")

	return Http.post(url, {
		clientId = self.clientId,
		kind = kind,
		message = message,
	})
end

function Client:readConsent(): Promise.TypedPromise<string?>
	Log.trace("Client request: consent read")

	local url = self:getUrl("consent/read")

	return Http.post(url, {
		clientId = self.clientId,
	}):andThen(function(response)
		local reply = response:decode()

		return reply and reply.decision or nil
	end)
end

function Client:clearConsent(): Promise.Promise
	Log.trace("Client request: consent clear")

	local url = self:getUrl("consent/clear")

	return Http.post(url, {
		clientId = self.clientId,
	})
end

function Client:open(instance: Types.Ref, line: number): Promise.Promise
	Log.trace("Client request: open")

	local url = self:getUrl("open")

	return Http.post(url, {
		instance = instance,
		line = line,
	})
end

return Client
