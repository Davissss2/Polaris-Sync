local Error = require(script.Error)
local PropertyDescriptor = require(script.PropertyDescriptor)

-- The reflection database is a multi-megabyte table. Building it at require time
-- delays the plugin loading for no reason, so it is only built on first use.
local database
local function getDatabase()
	if database == nil then
		database = require(script.database)
	end

	return database
end

-- Descriptors are resolved by walking the superclass chain and allocating a new
-- descriptor every time, and that happens for every property of every change.
local descriptorCache = {}
local defaultPropertiesCache = {}
local creatableCache = {}

local function computeCanonicalPropertyDescriptor(className, propertyName)
	local classes = getDatabase().Classes
	local currentClassName = className

	repeat
		local currentClass = classes[currentClassName]

		if currentClass == nil then
			return nil
		end

		local propertyData = currentClass.Properties[propertyName]
		if propertyData ~= nil then
			local canonicalData = propertyData.Kind.Canonical
			if canonicalData ~= nil then
				return PropertyDescriptor.fromRaw(propertyData, currentClassName, propertyName)
			end

			local aliasData = propertyData.Kind.Alias
			if aliasData ~= nil then
				return PropertyDescriptor.fromRaw(
					currentClass.Properties[aliasData.AliasFor],
					currentClassName,
					aliasData.AliasFor
				)
			end

			return nil
		end

		currentClassName = currentClass.Superclass
	until currentClassName == nil

	return nil
end

local function findCanonicalPropertyDescriptor(className, propertyName)
	local classCache = descriptorCache[className]

	if classCache == nil then
		classCache = {}
		descriptorCache[className] = classCache
	end

	-- `false` marks a miss, so unknown properties are not resolved twice
	local cached = classCache[propertyName]

	if cached ~= nil then
		return cached or nil
	end

	local descriptor = computeCanonicalPropertyDescriptor(className, propertyName) or false
	classCache[propertyName] = descriptor

	return descriptor or nil
end

local function getDefaultProperties(className)
	local cached = defaultPropertiesCache[className]

	if cached then
		return cached
	end

	local class = getDatabase().Classes[className]
	local properties = {}

	if class then
		for property, default in pairs(class.DefaultProperties) do
			local descriptor = findCanonicalPropertyDescriptor(className, property)

			if
				descriptor
				and (descriptor.scriptability == "ReadWrite" or descriptor.scriptability == "Custom")
				and next(default)
			then
				properties[property] = default
			end
		end
	end

	defaultPropertiesCache[className] = properties

	return properties
end

local function isCreatable(className)
	local cached = creatableCache[className]

	if cached ~= nil then
		return cached
	end

	local class = getDatabase().Classes[className]
	local creatable = class ~= nil
		and (table.find(class.Tags, "NotCreatable") or table.find(class.Tags, "Service")) == nil

	creatableCache[className] = creatable

	return creatable
end

local function readProperty(instance, propertyName)
	local descriptor = findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

	if descriptor == nil then
		local fullName = ("%s.%s"):format(instance.className, propertyName)

		return false, Error.new(Error.Kind.UnknownProperty, fullName)
	end

	return descriptor:read(instance)
end

local function writeProperty(instance, propertyName, value)
	local descriptor = findCanonicalPropertyDescriptor(instance.ClassName, propertyName)

	if descriptor == nil then
		local fullName = ("%s.%s"):format(instance.className, propertyName)

		return false, Error.new(Error.Kind.UnknownProperty, fullName)
	end

	return descriptor:write(instance, value)
end

return {
	isCreatable = isCreatable,
	readProperty = readProperty,
	writeProperty = writeProperty,
	findCanonicalPropertyDescriptor = findCanonicalPropertyDescriptor,
	getDefaultProperties = getDefaultProperties,
	Error = Error,
	EncodedValue = require(script.EncodedValue),
}
