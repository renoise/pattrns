# cycle  
* [global](#global)  
	* [Functions](#functions)  
		* [cycle](#cycle) ([`string`](../API/builtins/string.md)) `->` [`Cycle`](../API/cycle.md#Cycle)  
	* [Aliases](#aliases)  
		* [CycleMapFunction](#CycleMapFunction)  
		* [CycleMapGenerator](#CycleMapGenerator)  
		* [CycleMapNoteValue](#CycleMapNoteValue)  
		* [NoteValue](#NoteValue)  
		* [PlaybackState](#PlaybackState)  
* [Cycle](#Cycle)  
	* [Functions](#functions)  
		* [map](#map) ([*self*](../API/builtins/self.md), [`CycleMapFunction`](#CycleMapFunction) | [`CycleMapGenerator`](#CycleMapGenerator) | {  }) `->` [`Cycle`](../API/cycle.md#Cycle)  
	* [Aliases](#aliases)  
		* [CycleMapFunction](#CycleMapFunction)  
		* [CycleMapGenerator](#CycleMapGenerator)  
		* [CycleMapNoteValue](#CycleMapNoteValue)  
		* [NoteValue](#NoteValue)  
		* [PlaybackState](#PlaybackState)  
* [CycleMapContext](#CycleMapContext)  
	* [Properties](#properties)  
		* [playback](#playback) : [`PlaybackState`](#PlaybackState)  
		* [channel](#channel) : [`integer`](../API/builtins/integer.md)  
		* [step](#step) : [`integer`](../API/builtins/integer.md)  
		* [step_length](#step_length) : [`number`](../API/builtins/number.md)  
		* [trigger](#trigger) : [`Note`](../API/note.md#Note)[`?`](../API/builtins/nil.md)  
		* [parameter](#parameter) : [`table`](../API/builtins/table.md)`<`[`string`](../API/builtins/string.md), [`boolean`](../API/builtins/boolean.md) | [`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md)`>`  
		* [beats_per_min](#beats_per_min) : [`number`](../API/builtins/number.md)  
		* [beats_per_bar](#beats_per_bar) : [`integer`](../API/builtins/integer.md)  
		* [samples_per_sec](#samples_per_sec) : [`integer`](../API/builtins/integer.md)  
	* [Aliases](#aliases)  
		* [PlaybackState](#PlaybackState)  
# global { #global }
---
## Functions
### cycle(input : [`string`](../API/builtins/string.md)) { #cycle }
`->`[`Cycle`](../API/cycle.md#Cycle)  

> Create a note sequence from a Tidal Cycles mini-notation string.
> 
> `cycle` accepts a mini-notation as used by Tidal Cycles, with the following differences:
> * Stacks and random choices are valid without brackets (`a | b` is parsed as `[a | b]`)
> * `:` sets the instrument or remappable target instead of selecting samples but also 
>   allows setting note attributes such as instrument/volume/pan/delay (e.g. `c4:v0.1:p0.5`)
> * In bjorklund expressions, operators *within* and on the *right side* are not supported
>   (e.g. `bd(<3 2>, 8)` and `bd(3, 8)*2` are *not* supported)
> 
> [Tidal Cycles Reference](https://tidalcycles.org/docs/reference/mini_notation/)
> 
> #### examples:
>  ```lua
> --A chord sequence
> cycle("[c4, e4, g4] [e4, g4, b4] [g4, b4, d5] [b4, d5, f#5]")
> ```
> ```lua
> --Arpeggio pattern with variations
> cycle("<c4 e4 g4> <e4 g4> <g4 b4 d5> <b4 f5>")
> ```
> ```lua
> --Euclidean Rhythms
> cycle("c4(3,8) e4(5,8) g4(7,8)")
> ```
> ```lua
> --Map custom identifiers to notes
> cycle("bd(3,8)"):map({ bd = "c4 #1" })
>  ```
---
# Aliases
---
---
### CycleMapFunction { #CycleMapFunction }
 (context : [`CycleMapContext`](../API/cycle.md#CycleMapContext), value : [`string`](../API/builtins/string.md)) `->` [`CycleMapNoteValue`](#CycleMapNoteValue)  

---
### CycleMapGenerator { #CycleMapGenerator }
 (context : [`CycleMapContext`](../API/cycle.md#CycleMapContext), value : [`string`](../API/builtins/string.md)) `->` [`CycleMapFunction`](#CycleMapFunction)  

---
### CycleMapNoteValue { #CycleMapNoteValue }
[`NoteValue`](#NoteValue) | [`NoteValue`](#NoteValue)[`[]`](../API/builtins/array.md)  

---
### NoteValue { #NoteValue }
[`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md) | [`Note`](../API/note.md#Note) | [`NoteTable`](../API/note.md#NoteTable) | [`nil`](../API/builtins/nil.md)  

---
### PlaybackState { #PlaybackState }
`"running"` | `"seeking"`  
> ```lua
> -- - *seeking*: The pattern is auto-seeked to a target time. All events are discarded. Avoid
> --   unnecessary computations while seeking, and only maintain your generator's internal state.
> -- - *running*: The pattern is played back regularly. Events are emitted and audible.
> PlaybackState:
>     | "seeking"
>     | "running"
> ```
---  
# Cycle { #Cycle }
---
## Functions
### map([*self*](../API/builtins/self.md), map : [`CycleMapFunction`](#CycleMapFunction) | [`CycleMapGenerator`](#CycleMapGenerator) | {  }) { #map }
`->`[`Cycle`](../API/cycle.md#Cycle)  

> Map names in in the cycle to custom note events.
> 
> By default, strings in cycles are interpreted as notes, and integer values as MIDI note
> values. Custom identifiers such as "bd" are undefined and will result into a rest, when
> they are not mapped explicitly.
> 
> #### examples:
> ```lua
> --Using a static map table
> cycle("bd [bd, sn]"):map({
>   bd = "c4",
>   sn = "e4 #1 v0.2"
> })
> ```
> ```lua
> --Using a static map table with targets
> cycle("bd:1 <bd:5, bd:7>"):map({
>   -- instrument #1,5,7 are set additionally, as specified
>   bd = { key = "c4", volume = 0.5 },
> })
> ```
> ```lua
> --Using a dynamic map function
> cycle("4 5 4 <5 [4|6]>"):map(function(context, value)
>   -- emit a random note with 'value' as octave
>   return math.random(0, 11) + value * 12
> end)
> ```
> ```lua
> --Using a dynamic map function generator
> cycle("4 5 4 <4 [5|7]>"):map(function(init_context)
>   local notes = scale("c", "minor").notes
>   return function(context, value)
>     -- emit a 'cmin' note arp with 'value' as octave
>     local note = notes[math.imod(context.step, #notes)]
>     local octave = tonumber(value)
>     return { key = note + octave * 12 }
>   end
> end)
> ```
> ```lua
> --Using a dynamic map function to map values to chord degrees
> cycle("1 5 1 [6|7]"):map(function(init_context)
>   local cmin = scale("c", "minor")
>   return function(context, value)
>     return note(cmin:chord(tonumber(value)))
>   end
> end)
> ```
---
# Aliases
---
---
### CycleMapFunction { #CycleMapFunction }
 (context : [`CycleMapContext`](../API/cycle.md#CycleMapContext), value : [`string`](../API/builtins/string.md)) `->` [`CycleMapNoteValue`](#CycleMapNoteValue)  

---
### CycleMapGenerator { #CycleMapGenerator }
 (context : [`CycleMapContext`](../API/cycle.md#CycleMapContext), value : [`string`](../API/builtins/string.md)) `->` [`CycleMapFunction`](#CycleMapFunction)  

---
### CycleMapNoteValue { #CycleMapNoteValue }
[`NoteValue`](#NoteValue) | [`NoteValue`](#NoteValue)[`[]`](../API/builtins/array.md)  

---
### NoteValue { #NoteValue }
[`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md) | [`Note`](../API/note.md#Note) | [`NoteTable`](../API/note.md#NoteTable) | [`nil`](../API/builtins/nil.md)  

---
### PlaybackState { #PlaybackState }
`"running"` | `"seeking"`  
> ```lua
> -- - *seeking*: The pattern is auto-seeked to a target time. All events are discarded. Avoid
> --   unnecessary computations while seeking, and only maintain your generator's internal state.
> -- - *running*: The pattern is played back regularly. Events are emitted and audible.
> PlaybackState:
>     | "seeking"
>     | "running"
> ```
---  
# CycleMapContext { #CycleMapContext }
> Context passed to 'cycle:map` functions.
---
## Properties
### playback : [`PlaybackState`](#PlaybackState) { #playback }
> Specifies how the cycle currently is running.

### channel : [`integer`](../API/builtins/integer.md) { #channel }
> channel/voice index within the cycle. each channel in the cycle gets emitted and thus mapped
> separately, starting with the first channel index 1.

### step : [`integer`](../API/builtins/integer.md) { #step }
> Continues step counter for each channel, incrementing with each new mapped value in the cycle.
> Starts from 1 when the cycle starts running or after it got reset.

### step_length : [`number`](../API/builtins/number.md) { #step_length }
> step length fraction within the cycle, where 1 is the total duration of a single cycle run.

### trigger : [`Note`](../API/note.md#Note)[`?`](../API/builtins/nil.md) { #trigger }
> Note that triggered the pattern, if any. Usually will ne a monophic note.
> To access the raw note number value use: `context.trigger.notes[1].key`

### parameter : [`table`](../API/builtins/table.md)`<`[`string`](../API/builtins/string.md), [`boolean`](../API/builtins/boolean.md) | [`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md)`>` { #parameter }
> Current parameter values: parameter ids are keys, parameter values are values.
> To access a parameter with id `enabled` use: `context.parameter.enabled`

### beats_per_min : [`number`](../API/builtins/number.md) { #beats_per_min }
> Project's tempo in beats per minutes.

### beats_per_bar : [`integer`](../API/builtins/integer.md) { #beats_per_bar }
> Project's beats per bar settings - usually will be 4.

### samples_per_sec : [`integer`](../API/builtins/integer.md) { #samples_per_sec }
> Project's audio playback sample rate in samples per second.

---
# Aliases
---
---
### PlaybackState { #PlaybackState }
`"running"` | `"seeking"`  
> ```lua
> -- - *seeking*: The pattern is auto-seeked to a target time. All events are discarded. Avoid
> --   unnecessary computations while seeking, and only maintain your generator's internal state.
> -- - *running*: The pattern is played back regularly. Events are emitted and audible.
> PlaybackState:
>     | "seeking"
>     | "running"
> ```
---