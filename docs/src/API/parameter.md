# parameter  
* [Parameter](#Parameter)  
	* [Functions](#functions)  
		* [boolean](#boolean) ([`ParameterId`](#ParameterId), [`ParameterBooleanDefault`](#ParameterBooleanDefault), [`ParameterName`](#ParameterName)[`?`](../API/builtins/nil.md), [`ParameterDescription`](#ParameterDescription)[`?`](../API/builtins/nil.md)) `->` [`Parameter`](../API/parameter.md#Parameter)  
		* [integer](#integer) ([`ParameterId`](#ParameterId), [`ParameterIntegerDefault`](#ParameterIntegerDefault), [`ParameterIntegerRange`](#ParameterIntegerRange)[`?`](../API/builtins/nil.md), [`ParameterName`](#ParameterName)[`?`](../API/builtins/nil.md), [`ParameterDescription`](#ParameterDescription)[`?`](../API/builtins/nil.md)) `->` [`Parameter`](../API/parameter.md#Parameter)  
		* [number](#number) ([`ParameterId`](#ParameterId), [`ParameterNumberDefault`](#ParameterNumberDefault), [`ParameterNumberRange`](#ParameterNumberRange)[`?`](../API/builtins/nil.md), [`ParameterName`](#ParameterName)[`?`](../API/builtins/nil.md), [`ParameterDescription`](#ParameterDescription)[`?`](../API/builtins/nil.md)) `->` [`Parameter`](../API/parameter.md#Parameter)  
		* [enum](#enum) ([`ParameterId`](#ParameterId), [`ParameterEnumDefault`](#ParameterEnumDefault), [`string`](../API/builtins/string.md)[`[]`](../API/builtins/array.md), [`ParameterName`](#ParameterName)[`?`](../API/builtins/nil.md), [`ParameterDescription`](#ParameterDescription)[`?`](../API/builtins/nil.md)) `->` [`Parameter`](../API/parameter.md#Parameter)  
	* [Aliases](#aliases)  
		* [ParameterBooleanDefault](#ParameterBooleanDefault)  
		* [ParameterDescription](#ParameterDescription)  
		* [ParameterEnumDefault](#ParameterEnumDefault)  
		* [ParameterId](#ParameterId)  
		* [ParameterIntegerDefault](#ParameterIntegerDefault)  
		* [ParameterIntegerRange](#ParameterIntegerRange)  
		* [ParameterName](#ParameterName)  
		* [ParameterNumberDefault](#ParameterNumberDefault)  
		* [ParameterNumberRange](#ParameterNumberRange)  
# Parameter { #Parameter }
> Opaque parameter user data. Construct new parameters via the `parameter.XXX(...)`
> functions.
---
## Functions
### boolean(id : [`ParameterId`](#ParameterId), default : [`ParameterBooleanDefault`](#ParameterBooleanDefault), name : [`ParameterName`](#ParameterName)[`?`](../API/builtins/nil.md), description : [`ParameterDescription`](#ParameterDescription)[`?`](../API/builtins/nil.md)) { #boolean }
`->`[`Parameter`](../API/parameter.md#Parameter)  

> Creates an Parameter with "boolean" Lua type with the given default value
> and other optional properties.
### integer(id : [`ParameterId`](#ParameterId), default : [`ParameterIntegerDefault`](#ParameterIntegerDefault), range : [`ParameterIntegerRange`](#ParameterIntegerRange)[`?`](../API/builtins/nil.md), name : [`ParameterName`](#ParameterName)[`?`](../API/builtins/nil.md), description : [`ParameterDescription`](#ParameterDescription)[`?`](../API/builtins/nil.md)) { #integer }
`->`[`Parameter`](../API/parameter.md#Parameter)  

> Creates an Parameter with "integer" Lua type with the given default value
> and other optional properties.
### number(id : [`ParameterId`](#ParameterId), default : [`ParameterNumberDefault`](#ParameterNumberDefault), range : [`ParameterNumberRange`](#ParameterNumberRange)[`?`](../API/builtins/nil.md), name : [`ParameterName`](#ParameterName)[`?`](../API/builtins/nil.md), description : [`ParameterDescription`](#ParameterDescription)[`?`](../API/builtins/nil.md)) { #number }
`->`[`Parameter`](../API/parameter.md#Parameter)  

> Creates an Parameter with "number" Lua type with the given default value
> and other optional properties.
### enum(id : [`ParameterId`](#ParameterId), default : [`ParameterEnumDefault`](#ParameterEnumDefault), values : [`string`](../API/builtins/string.md)[`[]`](../API/builtins/array.md), name : [`ParameterName`](#ParameterName)[`?`](../API/builtins/nil.md), description : [`ParameterDescription`](#ParameterDescription)[`?`](../API/builtins/nil.md)) { #enum }
`->`[`Parameter`](../API/parameter.md#Parameter)  

> Creates an Parameter with a "string" Lua type with the given default value,
> set of valid values to choose from and other optional properties.
---
# Aliases
---
---
### ParameterBooleanDefault { #ParameterBooleanDefault }
[`boolean`](../API/builtins/boolean.md)  
> Default boolean value.
---
### ParameterDescription { #ParameterDescription }
[`string`](../API/builtins/string.md)  
> Optional long description of the parameter describing what the parameter does.
---
### ParameterEnumDefault { #ParameterEnumDefault }
[`string`](../API/builtins/string.md)  
> Default string value. Must be a valid string within the specified value set.
---
### ParameterId { #ParameterId }
[`string`](../API/builtins/string.md)  
> Unique id of the parameter. The id will be used in the `parameter` context table as key.
---
### ParameterIntegerDefault { #ParameterIntegerDefault }
[`integer`](../API/builtins/integer.md)  
> Default integer value. Must be in the specified value range.
---
### ParameterIntegerRange { #ParameterIntegerRange }
{ 1 : [`integer`](../API/builtins/integer.md), 2 : [`integer`](../API/builtins/integer.md) }  
> Optional value range. When undefined (0.0 - 1.0)
---
### ParameterName { #ParameterName }
[`string`](../API/builtins/string.md)  
> Optional name of the parameter as displayed to the user. When undefined, the id is used.
---
### ParameterNumberDefault { #ParameterNumberDefault }
[`number`](../API/builtins/number.md)  
> Default number value. Must be in the specified value range.
---
### ParameterNumberRange { #ParameterNumberRange }
{ 1 : [`number`](../API/builtins/number.md), 2 : [`number`](../API/builtins/number.md) }  
> Optional value range. When undefined (0 - 100)
---