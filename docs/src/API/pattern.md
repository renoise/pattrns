# pattern
<!-- toc -->
# Global<a name="Global"></a>  

---  
## Functions
### pattern(options : [`PatternOptions`](../API/pattern.md#PatternOptions))<a name="pattern"></a>
`->`[`userdata`](../API/builtins/userdata.md)  

> Create a new pattern with the given properties table:
> 
> #### examples:
> ```lua
> --trigger notes in an euclidean triplet pattern
> return pattern {
>   unit = "1/16",
>   resolution = 2/3,
>   pulse = pulse.euclidean(12, 16),
>   event = { "c4", "g4", { "c5 v0.7", "g5 v0.4" }, "a#4" }
> }
> ```
> ```lua
> -- trigger a chord sequence every 4 bars after 4 bars
> return pattern {
>   unit = "bars",
>   resolution = 4,
>   offset = 1,
>   event = {
>     note("c4'm"),
>     note("g3'm7"):transpose({ 0, 12, 0, 0 })
>   }
> }
> ```
> ```lua
> --trigger notes in a seeded, random subdivision pattern
> local seed = 23498
> return pattern {
>   unit = "1/8",
>   pulse = { 1, { 0, 1 }, 0, 0.3, 0.2, 1, { 0.5, 0.1, 1 }, 0.5 },
>   gate = function(init_context)
>     local rand = math.randomstate(seed)
>     return function(context)
>       return context.pulse_value > rand()
>     end
>   end,
>   event = { "c4" }
> }
> ```
> ```lua
> --trigger random notes in a seeded random pattern from a pentatonic scale
> local cmin = scale("c5", "pentatonic minor").notes
> return pattern {
>   unit = "1/16",
>   pulse = function(context)
>     return (context.pulse_step % 4 == 1) or (math.random() > 0.8)
>   end,
>   event = function(context)
>     return { key = cmin[math.random(#cmin)], volume = 0.7 }
>   end
> }
> ```
> ```lua
> --emit a tidal cycle every bar
> return pattern {
>   unit = "bars",
>   event = cycle("[c4 [f5 f4]*2]|[c4 [g5 g4]*3]")
> }
> ```  



---  
## Aliases  
### NoteValue<a name="NoteValue"></a>
[`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md) | [`Note`](../API/note.md#Note) | [`NoteTable`](../API/note.md#NoteTable) | [`nil`](../API/builtins/nil.md)  
  
  
### PulseValue<a name="PulseValue"></a>
[`boolean`](../API/builtins/boolean.md) | [`number`](../API/builtins/number.md) | [`boolean`](../API/builtins/boolean.md) | [`number`](../API/builtins/number.md) | `0` | `1` | [`PulseValue`](#PulseValue) | [`nil`](../API/builtins/nil.md)[] | `0` | `1` | [`nil`](../API/builtins/nil.md)  
> ```lua
> -- Single pulse value or a nested subdivision of pulses within a rhythm's pulse.
> PulseValue:
>     | 0
>     | 1
> ```  
  



# EventContext<a name="EventContext"></a>  
> Event related context passed to functions in 'emit'.  

---  
## Properties
### trigger : [`Note`](../API/note.md#Note)[`?`](../API/builtins/nil.md)<a name="trigger"></a>
> Note that triggered the pattern, if any. Usually will ne a monophic note.
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

### pulse_step : [`integer`](../API/builtins/integer.md)<a name="pulse_step"></a>
> Continues pulse counter, incrementing with each new **skipped or emitted pulse**.
> Unlike `step` in event this includes all pulses, so it also counts pulses which do
> not emit events. Starts from 1 when the pattern starts running or after it got reset.

### pulse_time_step : [`number`](../API/builtins/number.md)<a name="pulse_time_step"></a>
> Continues pulse time counter, incrementing with each new **skipped or emitted pulse**.
> Starts from 0 and increases with each new pulse by the pulse's step time duration.

### pulse_time : [`number`](../API/builtins/number.md)<a name="pulse_time"></a>
> Current pulse's step time as fraction of a full step in the pulse. For simple pulses this
> will be 1, for pulses in subdivisions this will be the reciprocal of the number of steps in
> the subdivision, relative to the parent subdivisions pulse step time.
> #### examples:
> ```lua
> {1, {1, 1}} --> step times: {1, {0.5, 0.5}}
> ```

### pulse_value : [`number`](../API/builtins/number.md)<a name="pulse_value"></a>
> Current pulse value. For binary pulses this will be 0 or 1, but it can be any number value.

### playback : [`PlaybackState`](#PlaybackState)<a name="playback"></a>
> Specifies how the pattern currently is running.

### step : [`integer`](../API/builtins/integer.md)<a name="step"></a>
> Continues step counter, incrementing with each new *emitted* pulse.
> Unlike `pulse_step` this does not include skipped, zero values pulses so it basically counts
> how often the event function already got called.
> Starts from 1 when the pattern starts running or is reset.

  



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
  



# GateContext<a name="GateContext"></a>  
> Pulse value context passed to functions in `gate` and `event`.  

---  
## Properties
### trigger : [`Note`](../API/note.md#Note)[`?`](../API/builtins/nil.md)<a name="trigger"></a>
> Note that triggered the pattern, if any. Usually will ne a monophic note.
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

### pulse_step : [`integer`](../API/builtins/integer.md)<a name="pulse_step"></a>
> Continues pulse counter, incrementing with each new **skipped or emitted pulse**.
> Unlike `step` in event this includes all pulses, so it also counts pulses which do
> not emit events. Starts from 1 when the pattern starts running or after it got reset.

### pulse_time_step : [`number`](../API/builtins/number.md)<a name="pulse_time_step"></a>
> Continues pulse time counter, incrementing with each new **skipped or emitted pulse**.
> Starts from 0 and increases with each new pulse by the pulse's step time duration.

### pulse_time : [`number`](../API/builtins/number.md)<a name="pulse_time"></a>
> Current pulse's step time as fraction of a full step in the pulse. For simple pulses this
> will be 1, for pulses in subdivisions this will be the reciprocal of the number of steps in
> the subdivision, relative to the parent subdivisions pulse step time.
> #### examples:
> ```lua
> {1, {1, 1}} --> step times: {1, {0.5, 0.5}}
> ```

### pulse_value : [`number`](../API/builtins/number.md)<a name="pulse_value"></a>
> Current pulse value. For binary pulses this will be 0 or 1, but it can be any number value.

  



# PatternOptions<a name="PatternOptions"></a>  
> Construction options for a new pattern.  

---  
## Properties
### unit : `"ms"` | `"seconds"` | `"bars"` | `"beats"` | `"1/1"` | `"1/2"` | `"1/4"` | `"1/8"` | `"1/16"` | `"1/32"` | `"1/64"`<a name="unit"></a>
> Base time unit of the pattern. Use `resolution` to apply an additional factor, in order to
> create other less common time bases.
> #### examples:
> ```lua
> -- slightly off beat pulse
> unit = "beats",
> resolution = 1.01
> ```
> ```lua
> -- triplet
> unit = "1/16",
> resolution = 2/3
> ```

### resolution : [`number`](../API/builtins/number.md)<a name="resolution"></a>
> Factor which is applied on `unit` to specify the final time resolution of the pattern.
> #### examples:
> ```lua
> -- slightly off beat pulse
> unit = "beats",
> resolution = 1.01
> ```
> ```lua
> -- triplet
> unit = "1/16",
> resolution = 2/3
> ```

### offset : [`number`](../API/builtins/number.md)<a name="offset"></a>
> Optional offset in `unit * resolution` time units. By default 0.
> When set, the pattern's event output will be delayed by the given offset value.
> #### examples:
> ```lua
> -- start emitting after 4*4 beats
> unit = "1/4",
> resolution = 4,
> offset = 4
> ```

### parameter : [`Parameter`](../API/parameter.md#Parameter)[]<a name="parameter"></a>
> Define optional parameters for the pattern. Parameters can dynamically
> change a patterns behavior everywhere where `context`s are passed, e.g. in `pulse`,
> `gate`, `event` or `cycle` map generator functions.
> 
> #### examples:
> ```lua
> -- trigger a single note as specified by parameter 'note'
> -- when parameter 'enabled' is true, else triggers nothing.
> return pattern {
>   parameter = {
>     parameter.boolean("enabled", true),
>     parameter.integer("note", 48, { 0, 127 })
>   },
>   event = function(context)
>     if context.parameter.enabled then -- boolean value
>       return note(context.parameter.note) -- integer value
>     else
>       return nil
>     end
>   end
> }
> ```

### pulse : [`boolean`](../API/builtins/boolean.md) | [`number`](../API/builtins/number.md) | `0` | `1` | [`PulseValue`](#PulseValue) | [`nil`](../API/builtins/nil.md)[] | (context : [`PulseContext`](../API/pattern.md#PulseContext)) `->` [`boolean`](../API/builtins/boolean.md) | [`number`](../API/builtins/number.md) | `0` | `1` | [`PulseValue`](#PulseValue) | [`nil`](../API/builtins/nil.md) | (context : [`PulseContext`](../API/pattern.md#PulseContext)) `->` (context : [`PulseContext`](../API/pattern.md#PulseContext)) `->` [`boolean`](../API/builtins/boolean.md) | [`number`](../API/builtins/number.md) | `0` | `1` | [`PulseValue`](#PulseValue) | [`nil`](../API/builtins/nil.md)<a name="pulse"></a>
> Defines the rhythmical part of the pattern. With the default `gate` implementation,
> each pulse with a value of `1` or `true` will cause an event from the `event` property
> to be triggered in the pattern's time unit. `0`, `false` or `nil` values do not trigger.
> 
> Pulses may contain subdivisions, sub-tables of pulses, to "cram" multiple pulses
> into a single pulse's time interval. This way more complex rhythmical patterns can
> be created.
> 
> When no pulse is defined, a constant pulse value of `1` is triggered.
> 
> Just like the `event` property, pulses can either be a static array of values
> or a function or generators which produces pulse values dynamically.
> 
> #### examples:
> ```lua
> -- static pattern
> pulse = { 1, 0, 0, 1 }
> ```
> ```lua
> -- "cram" pulses into a single pulse slot via sub-divisions
> pulse = { 1, { 1, 1, 1 } }
> ```
> ```lua
> -- pulses created via the "pulse" lib
> pulse = pulse.from{ 1, 0 } * 3 + { 1, 1 }
> pulse = pulse.euclidean(7, 16, 2)
> ```
> ```lua
> -- stateless pulse function
> pulse = function(context)
>   return math.random(0, 1)
> end
> ```
> ```lua
> -- stateful pulse function
> pulse = function(init_context)
>   local triggers = table.from{ 0, 6, 10 }
>   return function(context)
>     local step = (context.pulse_step - 1) % 16
>     return triggers:contains(step)
>   end
> end
> ```

### repeats : [`boolean`](../API/builtins/boolean.md) | [`integer`](../API/builtins/integer.md)<a name="repeats"></a>
> If and how many times a pattern should repeat. When 0 or false, the pattern does not repeat
> and plays back only once. When true, the pattern repeats endlessly, which is the default.
> When a number > 0, this specifies the number of times the pattern repeats until it stops.
> 
> Note: When `pulse` is a function or iterator, the repeat count is the number of
> *function calls or iteration steps*. When the pattern is a pulse array, this is the number of
> times the whole pulse pattern gets repeated.
> 
> #### examples:
> ```lua
> -- one-shot
> repeat = 0
> -- also a one-shot
> repeat = false
> -- play the pattern 4 times
> repeat = 3
> -- play & repeat forever (default)
> repeat = true
> ```

### gate : (context : [`GateContext`](../API/pattern.md#GateContext)) `->` [`boolean`](../API/builtins/boolean.md) | (context : [`GateContext`](../API/pattern.md#GateContext)) `->` (context : [`GateContext`](../API/pattern.md#GateContext)) `->` [`boolean`](../API/builtins/boolean.md)<a name="gate"></a>
> Optional pulse train filter function which filters events between the pulse and event emitter.
> By default a threshold gate, which passes all pulse values greater than zero.
> 
> Functions return true when a pulse value should be passed, and false when the event
> emitter should be skipped.
> 
> #### examples:
> ```lua
> -- probability gate: skips all 0s, passes all 1s. pulse values in range (0, 1) are
> -- maybe passed, using the pulse value as probability.
> gate = function(context)
>   return context.pulse_value > math.random()
> end
> ```
> ```lua
> -- threshold gate: skips all pulse values below a given threshold value
> gate = function(context)
>   return context.pulse_value > 0.5
> end
> ```

### event : [`Cycle`](../API/cycle.md#Cycle) | [`Sequence`](../API/sequence.md#Sequence) | [`Note`](../API/note.md#Note) | [`NoteValue`](#NoteValue) | [`NoteValue`](#NoteValue)[] | (context : [`EventContext`](../API/pattern.md#EventContext)) `->` [`NoteValue`](#NoteValue) | (context : [`EventContext`](../API/pattern.md#EventContext)) `->` (context : [`EventContext`](../API/pattern.md#EventContext)) `->` [`NoteValue`](#NoteValue)<a name="event"></a>
> Specify the event values of the pattern. For every pulse in the pulse pattern, an event
> is picked from the specified event sequence. When the end of the sequence is reached, it starts
> again from the beginning.
> 
> To generate events dynamically, you can pass a function or a function iterator, instead of a
> static array or sequence of notes.
> 
> Events can also be generated via a tidal cycle mini-notation. Cycles are repeated endlessly
> by default, and have the duration of a single step in the patterns. Pulses can be used to
> sequence cycles too.
> 
> #### examples:
> ```lua
> -- a sequence of c4, g4
> event = {"c4", "g4"}
> ```
> ```lua
> -- a chord of c4, d#4, g4
> event = {{"c4", "d#4", "g4"}} -- or {"c4'min"}
> ```
> ```lua
> -- a sequence of c4, g4 with volume 0.5
> event = sequence{"c4", "g4"}:volume(0.5)
> ```
> ```lua
> -- stateless generator function
> event = function(context)
>   return 48 + math.random(1, 4) * 5
> end
> ```
> ```lua
> -- stateful generator function
> event = function(init_context)
>   local count, step, notes = 1, 2, scale("c5", "minor").notes
>   return function(context)
>     local key = notes[count]
>     count = (count + step - 1) % #notes + 1
>     return { key = key, volume = 0.5 }
>   end
> end
> ```
> ```lua
> -- a note pattern
> local tritone = scale("c5", "tritone")
> --[...]
> event = pulse.from(tritone:chord(1, 4)):euclidean(6) +
>   pulse.from(tritone:chord(5, 4)):euclidean(6)
> ```
> ```lua
> -- a tidal cycle
> event = cycle("<[a3 c4 e4 a4]*3 [d4 g3 g4 c4]>"),
> ```

  



---  
## Aliases  
### NoteValue<a name="NoteValue"></a>
[`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md) | [`Note`](../API/note.md#Note) | [`NoteTable`](../API/note.md#NoteTable) | [`nil`](../API/builtins/nil.md)  
  
  
### PulseValue<a name="PulseValue"></a>
[`boolean`](../API/builtins/boolean.md) | [`number`](../API/builtins/number.md) | [`boolean`](../API/builtins/boolean.md) | [`number`](../API/builtins/number.md) | `0` | `1` | [`PulseValue`](#PulseValue) | [`nil`](../API/builtins/nil.md)[] | `0` | `1` | [`nil`](../API/builtins/nil.md)  
> ```lua
> -- Single pulse value or a nested subdivision of pulses within a rhythm's pulse.
> PulseValue:
>     | 0
>     | 1
> ```  
  



# PulseContext<a name="PulseContext"></a>  
> Pulse timing context passed to functions in `pulse` and `gate`.  

---  
## Properties
### trigger : [`Note`](../API/note.md#Note)[`?`](../API/builtins/nil.md)<a name="trigger"></a>
> Note that triggered the pattern, if any. Usually will ne a monophic note.
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

### pulse_step : [`integer`](../API/builtins/integer.md)<a name="pulse_step"></a>
> Continues pulse counter, incrementing with each new **skipped or emitted pulse**.
> Unlike `step` in event this includes all pulses, so it also counts pulses which do
> not emit events. Starts from 1 when the pattern starts running or after it got reset.

### pulse_time_step : [`number`](../API/builtins/number.md)<a name="pulse_time_step"></a>
> Continues pulse time counter, incrementing with each new **skipped or emitted pulse**.
> Starts from 0 and increases with each new pulse by the pulse's step time duration.

  



