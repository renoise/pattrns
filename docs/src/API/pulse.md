# pulse  
* [Pulse](#Pulse)  
	* [Functions](#functions)  
		* [new](#new) ([`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md), [`PulseTableValue`](#PulseTableValue) |  (index : [`integer`](../API/builtins/integer.md)) `->` [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [from](#from) (...[`PulseTableValue`](#PulseTableValue) | [`PulseTableValue`](#PulseTableValue)[`[]`](../API/builtins/array.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [copy](#copy) ([`Pulse`](../API/pulse.md#Pulse)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [distributed](#distributed) ([`integer`](../API/builtins/integer.md) | [`table`](../API/builtins/table.md), [`integer`](../API/builtins/integer.md), [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md), [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [euclidean](#euclidean) ([`integer`](../API/builtins/integer.md) | [`table`](../API/builtins/table.md), [`integer`](../API/builtins/integer.md), [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md), [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [unpack](#unpack) ([`Pulse`](../API/pulse.md#Pulse)) `->` [`PulseTableValue`](#PulseTableValue)  
		* [subrange](#subrange) ([`Pulse`](../API/pulse.md#Pulse), [`integer`](../API/builtins/integer.md), [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md), [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [take](#take) ([`Pulse`](../API/pulse.md#Pulse), [`integer`](../API/builtins/integer.md), [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [clear](#clear) ([`Pulse`](../API/pulse.md#Pulse)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [init](#init) ([`Pulse`](../API/pulse.md#Pulse), [`PulseTableValue`](#PulseTableValue) |  (index : [`integer`](../API/builtins/integer.md)) `->` [`PulseTableValue`](#PulseTableValue), [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [map](#map) ([`Pulse`](../API/pulse.md#Pulse),  (index : [`integer`](../API/builtins/integer.md), value : [`PulseTableValue`](#PulseTableValue)) `->` [`PulseTableValue`](#PulseTableValue)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [reverse](#reverse) ([`Pulse`](../API/pulse.md#Pulse)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [rotate](#rotate) ([`Pulse`](../API/pulse.md#Pulse), [`integer`](../API/builtins/integer.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [push_back](#push_back) ([`Pulse`](../API/pulse.md#Pulse), ...[`PulseTableValue`](#PulseTableValue)[`[]`](../API/builtins/array.md) | [`PulseTableValue`](#PulseTableValue)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [pop_back](#pop_back) ([`Pulse`](../API/pulse.md#Pulse)) `->` [`PulseTableValue`](#PulseTableValue)  
		* [repeat_n](#repeat_n) ([`Pulse`](../API/pulse.md#Pulse), [`integer`](../API/builtins/integer.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [spread](#spread) ([`Pulse`](../API/pulse.md#Pulse), [`number`](../API/builtins/number.md), [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) `->` [`Pulse`](../API/pulse.md#Pulse)  
		* [tostring](#tostring) ([`Pulse`](../API/pulse.md#Pulse)) `->` [`string`](../API/builtins/string.md)  
	* [Aliases](#aliases)  
		* [PulseTableValue](#PulseTableValue)  
# Pulse { #Pulse }
> Table with helper functions to ease creating rhythmic patterns.
> 
> #### examples:
> ```lua
> -- using + and * operators to combine patterns
> pulse.from{ 0, 1 } * 3 + { 1, 0 }
> ```
> ```lua
> -- repeating, spreading and subsets
> pulse.from{ 0, 1, { 1, 1 } }:repeat_n(4):spread(1.25):take(16)
> ```
> ```lua
> -- euclidean patterns
> pulse.euclidean(12, 16)
> pulse.from{ 1, 0.5, 1, 1 }:euclidean(12)
> ```
> ```lua
> -- generate/init from functions
> pulse.new(8):init(1) --> 1,1,1,1,1,1,1,1
> pulse.new(12):init(function() return math.random(0.5, 1.0) end )
> pulse.new(16):init(scale("c", "minor").notes_iter())
> ```
> ```lua
> -- generate pulses with note values
> pulse.from{ "c4", "g4", "a4" } * 7 + { "a4", "g4", "c4" }
> ```
> ```lua
> -- generate chords from degree values
> pulse.from{ 1, 5, 6, 4 }:map(function(index, degree)
>   return scale("c", "minor"):chord(degree)
> end)
> ```
---
## Functions
### new(length : [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md), value : [`PulseTableValue`](#PulseTableValue) |  (index : [`integer`](../API/builtins/integer.md)) `->` [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) { #new }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Create a new empty pulse table or a pulse table with the given length and value.
> 
> 
> #### examples:
> ```lua
> pulse.new(4) --> {0,0,0,0}
> pulse.new(4, 1) --> {1,1,1,1}
> pulse.new(4, function() return math.random() end)
> ```
### from(...[`PulseTableValue`](#PulseTableValue) | [`PulseTableValue`](#PulseTableValue)[`[]`](../API/builtins/array.md)) { #from }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Create a new pulse table from an existing set of values or tables.
> When passing tables, those will be flattened.
> 
> #### examples:
> ```lua
> pulse.from(1,0,1,0) --> {1,0,1,0}
> pulse.from({1,0},{1,0}) --> {1,0,1,0}
> ```
### copy(self : [`Pulse`](../API/pulse.md#Pulse)) { #copy }
`->`[`Pulse`](../API/pulse.md#Pulse)  

>  create a shallow-copy of the given pulse table (or self)
> 
> #### examples:
> ```lua
> local p = pulse.from(1, 0)
> local p2 = p:copy() --> {1,0}
> ```
### distributed(steps : [`integer`](../API/builtins/integer.md) | [`table`](../API/builtins/table.md), length : [`integer`](../API/builtins/integer.md), offset : [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md), empty_value : [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) { #distributed }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Create an new pulse table or spread and existing pulse evenly within the given length.
> Similar, but not exactly like `euclidean`.
> 
> Shortcut for `pulse.from{1,1,1}:spread(length / #self):rotate(offset)`
> 
> #### examples:
> ```lua
> pulse.distributed(3, 8) --> {1,0,0,1,0,1,0}
> pulse.from{1,1}:distributed(4, 1) --> {0,1,0,1}
> ```
### euclidean(steps : [`integer`](../API/builtins/integer.md) | [`table`](../API/builtins/table.md), length : [`integer`](../API/builtins/integer.md), offset : [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md), empty_value : [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) { #euclidean }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Create a new euclidean rhythm pulse table with the given pulses or number of new pulses
> in the given length. Optionally rotate the contents too.
> [Euclidean Rhythm](https://en.wikipedia.org/wiki/Euclidean_rhythm)
> 
> #### examples:
> ```lua
> pulse.euclidean(3, 8)
>  --> {1,0,0,1,0,0,1,0}
> pulse.from{"x", "x", "x"}:euclidean(8, 0, "-")
>  --> {"x","-","-","x","-","-","x","-"}
> ```
### unpack(self : [`Pulse`](../API/pulse.md#Pulse)) { #unpack }
`->`... : [`PulseTableValue`](#PulseTableValue)  

> Shortcut for table.unpack(pulse): returns elements from this pulse as var args.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,2,3,4}
> local v1, v2, v3, v4 = p:unpack()
> ```
### subrange(self : [`Pulse`](../API/pulse.md#Pulse), i : [`integer`](../API/builtins/integer.md), j : [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md), empty_value : [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) { #subrange }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Fetch a sub-range from the pulse table as new pulse table.
> When the given length is past end of this pulse it is filled up with empty values.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,2,3,4}
> p = p:subrange(2,3) --> {2,3}
> p = p:subrange(1,4,"X") --> {2,3,"X","X"}
> ```
### take(self : [`Pulse`](../API/pulse.md#Pulse), length : [`integer`](../API/builtins/integer.md), empty_value : [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) { #take }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Get first n items from the pulse as new pulse table.
> When the given length is past end of this pulse its filled up with empty values.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,2,3,4}
> p = p:take(2) --> {1,2}
> p = p:take(4, "") --> {1,2,"",""}
> ```
### clear(self : [`Pulse`](../API/pulse.md#Pulse)) { #clear }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Clear a pulse table, remove all its contents.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,0}
> p:clear() --> {}
> ```
### init(self : [`Pulse`](../API/pulse.md#Pulse), value : [`PulseTableValue`](#PulseTableValue) |  (index : [`integer`](../API/builtins/integer.md)) `->` [`PulseTableValue`](#PulseTableValue), length : [`integer`](../API/builtins/integer.md)[`?`](../API/builtins/nil.md)) { #init }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Fill pulse table with the given value or generator function in the given length.
> 
> #### examples:
> ```lua
> local p = pulse.from{0,0}
> p:init(1) --> {1,1}
> p:init("X", 3) --> {"X","X", "X"}
> p:init(function(i) return math.random() end, 3)
> ```
### map(self : [`Pulse`](../API/pulse.md#Pulse), fun :  (index : [`integer`](../API/builtins/integer.md), value : [`PulseTableValue`](#PulseTableValue)) `->` [`PulseTableValue`](#PulseTableValue)) { #map }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Apply the given function to every item in the pulse table.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,3,5}
> p:map(function(k, v)
>   return scale("c", "minor"):degree(v)
> end) --> {48, 51, 55}
> ```
### reverse(self : [`Pulse`](../API/pulse.md#Pulse)) { #reverse }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Invert the order of items in the pulse table.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,2,3}
> p:reverse() --> {3,2,1}
> ```
### rotate(self : [`Pulse`](../API/pulse.md#Pulse), amount : [`integer`](../API/builtins/integer.md)) { #rotate }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Shift contents by the given amount to the left (negative amount) or right.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,0,0}
> p:rotate(1) --> {0,1,0}
> p:rotate(-2) --> {0,0,1}
> ```
### push_back(self : [`Pulse`](../API/pulse.md#Pulse), ...[`PulseTableValue`](#PulseTableValue)[`[]`](../API/builtins/array.md) | [`PulseTableValue`](#PulseTableValue)) { #push_back }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Push a single or multiple items or other pulse contents to the end of the pulse.
> Note: When passing array alike tables or patterns, they will be *unpacked*.
> 
> #### examples:
> ```lua
> local p = pulse.new()
> p:push_back(1) --> {1}
> p:push_back(2,3) --> {1,2,3}
> p:push_back{4} --> {1,2,3,4}
> p:push_back({5,{6,7}) --> {1,2,3,4,5,6,7}
> ```
### pop_back(self : [`Pulse`](../API/pulse.md#Pulse)) { #pop_back }
`->`[`PulseTableValue`](#PulseTableValue)  

> Remove an entry from the back of the pulse table. returns the removed item.
> 
> #### examples:
> ```lua
> local p = pulse.from({1,2})
> p:pop_back() --> {1}
> p:pop_back() --> {}
> p:pop_back() --> {}
> ```
### repeat_n(self : [`Pulse`](../API/pulse.md#Pulse), count : [`integer`](../API/builtins/integer.md)) { #repeat_n }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Repeat contents of the pulse table n times.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,2,3}
> patterns:repeat_n(2) --> {1,2,3,1,2,3}
> ```
### spread(self : [`Pulse`](../API/pulse.md#Pulse), amount : [`number`](../API/builtins/number.md), empty_value : [`PulseTableValue`](#PulseTableValue)[`?`](../API/builtins/nil.md)) { #spread }
`->`[`Pulse`](../API/pulse.md#Pulse)  

> Expand (with amount > 1) or shrink (amount < 1) the length of the pulse table by
> the given factor, spreading allowed content evenly and filling gaps with 0 or the
> given empty value.
> 
> #### examples:
> ```lua
> local p = pulse.from{1,1}
> p:spread(2) --> {1,0,1,0}
> p:spread(1/2) --> {1,1}
> ```
### tostring(self : [`Pulse`](../API/pulse.md#Pulse)) { #tostring }
`->`[`string`](../API/builtins/string.md)  

> Serialze a pulse table for display/debugging purposes.
> 
> #### examples:
> ```lua
> pulse.euclidean(3, 8):tostring() --> "{1, 0, 0, 1, 0, 0, 1, 0}"
> ```
---
# Aliases
---
---
### PulseTableValue { #PulseTableValue }
[`boolean`](../API/builtins/boolean.md) | [`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md) | [`table`](../API/builtins/table.md)  
> Valid pulse value in a pulse table
---