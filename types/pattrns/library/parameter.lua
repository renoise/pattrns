---@meta
error("Do not try to execute this file. It's just a type definition file.")
---
---Part of the pattrns crate: Defines LuaLS annotations for the pattrns Parameter class.
---

----------------------------------------------------------------------------------------------------

---Unique id of the parameter. The id will be used in the `parameter` context table as key.
---@alias ParameterId string

---Default boolean value.
---@alias ParameterBooleanDefault boolean
---Default integer value. Must be in the specified value range.
---@alias ParameterIntegerDefault integer
---Default number value. Must be in the specified value range.
---@alias ParameterNumberDefault number
---Default string value. Must be a valid string within the specified value set.
---@alias ParameterEnumDefault string

---Optional value range. When undefined (0.0 - 1.0)
---@alias ParameterIntegerRange { [1]: integer, [2]: integer }
---Optional value range. When undefined (0 - 100)
---@alias ParameterNumberRange { [1]: number, [2]: number }

---Optional name of the parameter as displayed to the user. When undefined, the id is used.
---@alias ParameterName string

---Optional long description of the parameter describing what the parameter does.
---@alias ParameterDescription string


----------------------------------------------------------------------------------------------------

---Opaque parameter user data. Construct new parameters via the `parameter.XXX(...)`
---functions.
---@see TriggerContext.parameter
---@class Parameter : userdata
local Parameter = {}

----------------------------------------------------------------------------------------------------

---Contains functions to construct new parameters. Parameter values can be accessed
---via function `contexts` in `pulse`, `gate` and `event` functions or generators.
---@class Parameter
parameter = {}

---Creates an Parameter with "boolean" Lua type with the given default value
---and other optional properties.
---@param id ParameterId
---@param default ParameterBooleanDefault
---@param name ParameterName?
---@param description ParameterDescription?
---@return Parameter
function parameter.boolean(id, default, name, description) end

---Creates an Parameter with "integer" Lua type with the given default value
---and other optional properties.
---@param id ParameterId
---@param default ParameterIntegerDefault
---@param range ParameterIntegerRange?
---@param name ParameterName?
---@param description ParameterDescription?
---@return Parameter
function parameter.integer(id, default, range, name, description) end

---Creates an Parameter with "number" Lua type with the given default value
---and other optional properties.
---@param id ParameterId
---@param default ParameterNumberDefault
---@param range ParameterNumberRange?
---@param name ParameterName?
---@param description ParameterDescription?
---@return Parameter
function parameter.number(id, default, range, name, description) end

---Creates an Parameter with a "string" Lua type with the given default value,
---set of valid values to choose from and other optional properties.
---@param id ParameterId
---@param default ParameterEnumDefault
---@param values string[]
---@param name ParameterName?
---@param description ParameterDescription?
---@return Parameter
function parameter.enum(id, default, values, name, description) end
