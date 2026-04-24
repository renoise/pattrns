# cycle
<!-- toc -->
# Global<a name="Global"></a>  

---  
## Functions
### cycle(input : [`string`](../API/builtins/string.md))<a name="cycle"></a>
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
## Aliases  
### CycleMapFunction<a name="CycleMapFunction"></a>
(context : [`CycleMapContext`](../API/cycle.md#CycleMapContext), value : [`string`](../API/builtins/string.md)) `->` [`CycleMapNoteValue`](#CycleMapNoteValue)  
  
  
### CycleMapGenerator<a name="CycleMapGenerator"></a>
(context : [`CycleMapContext`](../API/cycle.md#CycleMapContext), value : [`string`](../API/builtins/string.md)) `->` [`CycleMapFunction`](#CycleMapFunction)  
  
  
### CycleMapNoteValue<a name="CycleMapNoteValue"></a>
[`NoteValue`](#NoteValue) | [`NoteValue`](#NoteValue)[]  
  
  
### CycleMapTable<a name="CycleMapTable"></a>
{  }  
  
  
### CycleVarFunction<a name="CycleVarFunction"></a>
(context : [`CycleVarContext`](../API/cycle.md#CycleVarContext)) `->` [`CycleVarTable`](#CycleVarTable)  
  
  
### CycleVarGenerator<a name="CycleVarGenerator"></a>
(context : [`CycleVarContext`](../API/cycle.md#CycleVarContext)) `->` [`CycleVarFunction`](#CycleVarFunction)  
  
  
### CycleVarTable<a name="CycleVarTable"></a>
{  }  
  
  
### NoteValue<a name="NoteValue"></a>
[`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md) | [`Note`](../API/note.md#Note) | [`NoteTable`](../API/note.md#NoteTable) | [`nil`](../API/builtins/nil.md)  
  
  
### PlaybackState<a name="PlaybackState"></a>
`"running"` | `"seeking"`  
> ```lua
> -- - *seeking*: The pattern is auto-seeked to a target time. All events are discarded. Avoid
> --   unnecessary computations while seeking, and only maintain your generator's internal state.
> -- - *running*: The pattern is played back regularly. Events are emitted and audible.
> PlaybackState:
>     | "seeking"
>     | "running"
> ```  
  



# Cycle<a name="Cycle"></a>  

---  
## Functions
### map([*self*](../API/builtins/self.md), map : [`CycleMapFunction`](#CycleMapFunction) | [`CycleMapGenerator`](#CycleMapGenerator) | [`CycleMapTable`](#CycleMapTable))<a name="map"></a>
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
### var([*self*](../API/builtins/self.md), variables : [`CycleVarFunction`](#CycleVarFunction) | [`CycleVarGenerator`](#CycleVarGenerator) | [`CycleVarTable`](#CycleVarTable))<a name="var"></a>
`->`[`Cycle`](../API/cycle.md#Cycle)  

> Assign variables to be used inside the main cycle script via `$name` notation
> 
> By default any parameter you define will be available to the cycle as a variable
> but you can override and declare additional variables by passing in a table of them
> or passing a function that returns such a table.
> 
> #### examples:
> ```lua
> --Using a static table
> cycle("$a $a $b $a $b $b"):var({
>   a = "c a f e"
>   b = "e f a c"
> })
> ```  



---  
## Aliases  
### CycleMapFunction<a name="CycleMapFunction"></a>
(context : [`CycleMapContext`](../API/cycle.md#CycleMapContext), value : [`string`](../API/builtins/string.md)) `->` [`CycleMapNoteValue`](#CycleMapNoteValue)  
  
  
### CycleMapGenerator<a name="CycleMapGenerator"></a>
(context : [`CycleMapContext`](../API/cycle.md#CycleMapContext), value : [`string`](../API/builtins/string.md)) `->` [`CycleMapFunction`](#CycleMapFunction)  
  
  
### CycleMapNoteValue<a name="CycleMapNoteValue"></a>
[`NoteValue`](#NoteValue) | [`NoteValue`](#NoteValue)[]  
  
  
### CycleMapTable<a name="CycleMapTable"></a>
{  }  
  
  
### CycleVarFunction<a name="CycleVarFunction"></a>
(context : [`CycleVarContext`](../API/cycle.md#CycleVarContext)) `->` [`CycleVarTable`](#CycleVarTable)  
  
  
### CycleVarGenerator<a name="CycleVarGenerator"></a>
(context : [`CycleVarContext`](../API/cycle.md#CycleVarContext)) `->` [`CycleVarFunction`](#CycleVarFunction)  
  
  
### CycleVarTable<a name="CycleVarTable"></a>
{  }  
  
  
### NoteValue<a name="NoteValue"></a>
[`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md) | [`Note`](../API/note.md#Note) | [`NoteTable`](../API/note.md#NoteTable) | [`nil`](../API/builtins/nil.md)  
  
  
### PlaybackState<a name="PlaybackState"></a>
`"running"` | `"seeking"`  
> ```lua
> -- - *seeking*: The pattern is auto-seeked to a target time. All events are discarded. Avoid
> --   unnecessary computations while seeking, and only maintain your generator's internal state.
> -- - *running*: The pattern is played back regularly. Events are emitted and audible.
> PlaybackState:
>     | "seeking"
>     | "running"
> ```  
  



# CycleMapContext<a name="CycleMapContext"></a>  
> Context passed to 'cycle:map` functions.  

---  
## Properties
### playback : [`PlaybackState`](#PlaybackState)<a name="playback"></a>
> Specifies how the cycle currently is running.

### channel : [`integer`](../API/builtins/integer.md)<a name="channel"></a>
> channel/voice index within the cycle. each channel in the cycle gets emitted and thus mapped
> separately, starting with the first channel index 1.

### step : [`integer`](../API/builtins/integer.md)<a name="step"></a>
> Continues step counter for each channel, incrementing with each new mapped value in the cycle.
> Starts from 1 when the cycle starts running or after it got reset.

### step_length : [`number`](../API/builtins/number.md)<a name="step_length"></a>
> step length fraction within the cycle, where 1 is the total duration of a single cycle run.

### trigger : [`Note`](../API/note.md#Note)[`?`](../API/builtins/nil.md)<a name="trigger"></a>
> Note that triggered the pattern, if any. Usually will be a monophonic note.
> To access the raw note number value use: `context.trigger.notes[1].key`

### parameter : table<[`string`](../API/builtins/string.md), [`boolean`](../API/builtins/boolean.md) | [`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md)><a name="parameter"></a>
> Current parameter values: parameter ids are keys, parameter values are values.
> To access a parameter with id `enabled` use: `context.parameter.enabled`

### beats_per_min : [`number`](../API/builtins/number.md)<a name="beats_per_min"></a>
> Project's tempo in beats per minutes.

### beats_per_bar : [`integer`](../API/builtins/integer.md)<a name="beats_per_bar"></a>
> Project's beats per bar settings - usually will be 4.

### samples_per_sec : [`integer`](../API/builtins/integer.md)<a name="samples_per_sec"></a>
> Project's audio playback sample rate in samples per second.

  



---  
## Aliases  
### PlaybackState<a name="PlaybackState"></a>
`"running"` | `"seeking"`  
> ```lua
> -- - *seeking*: The pattern is auto-seeked to a target time. All events are discarded. Avoid
> --   unnecessary computations while seeking, and only maintain your generator's internal state.
> -- - *running*: The pattern is played back regularly. Events are emitted and audible.
> PlaybackState:
>     | "seeking"
>     | "running"
> ```  
  



# CycleVarContext<a name="CycleVarContext"></a>  
> Context passed to 'cycle:var` functions.  

---  
## Properties
### playback : [`PlaybackState`](#PlaybackState)<a name="playback"></a>
> Specifies how the cycle currently is running.

### iteration : [`integer`](../API/builtins/integer.md)<a name="iteration"></a>
> Iteration counter for the cycle that increases once per the whole cycle's output
> Starts from 1 when the cycle starts running or after it got reset.

### trigger : [`Note`](../API/note.md#Note)[`?`](../API/builtins/nil.md)<a name="trigger"></a>
> Note that triggered the pattern, if any. Usually will be a monophonic note.
> To access the raw note number value use: `context.trigger.notes[1].key`

### parameter : table<[`string`](../API/builtins/string.md), [`boolean`](../API/builtins/boolean.md) | [`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md)><a name="parameter"></a>
> Current parameter values: parameter ids are keys, parameter values are values.
> To access a parameter with id `enabled` use: `context.parameter.enabled`

### beats_per_min : [`number`](../API/builtins/number.md)<a name="beats_per_min"></a>
> Project's tempo in beats per minutes.

### beats_per_bar : [`integer`](../API/builtins/integer.md)<a name="beats_per_bar"></a>
> Project's beats per bar settings - usually will be 4.

### samples_per_sec : [`integer`](../API/builtins/integer.md)<a name="samples_per_sec"></a>
> Project's audio playback sample rate in samples per second.

  



---  
## Aliases  
### PlaybackState<a name="PlaybackState"></a>
`"running"` | `"seeking"`  
> ```lua
> -- - *seeking*: The pattern is auto-seeked to a target time. All events are discarded. Avoid
> --   unnecessary computations while seeking, and only maintain your generator's internal state.
> -- - *running*: The pattern is played back regularly. Events are emitted and audible.
> PlaybackState:
>     | "seeking"
>     | "running"
> ```  
  



