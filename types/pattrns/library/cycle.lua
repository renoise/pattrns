---@meta
error("Do not try to execute this file. It's just a type definition file.")
---
---Part of the pattrns crate: Defines LuaLS annotations for the pattrns Cycle class.
---

----------------------------------------------------------------------------------------------------

---Context passed to 'cycle:map` functions.
---@class CycleMapContext : TimeContext
---
---Specifies how the cycle currently is running.
---@field playback PlaybackState
---channel/voice index within the cycle. each channel in the cycle gets emitted and thus mapped
---separately, starting with the first channel index 1.
---@field channel integer
---Continues step counter for each channel, incrementing with each new mapped value in the cycle.
---Starts from 1 when the cycle starts running or after it got reset.
---@field step integer
---step length fraction within the cycle, where 1 is the total duration of a single cycle run.
---@field step_length number

----------------------------------------------------------------------------------------------------

---@class Cycle
local Cycle = {}

----------------------------------------------------------------------------------------------------

---@alias CycleMapNoteValue NoteValue|(NoteValue[])|Note
---@alias CycleMapFunction fun(context: CycleMapContext, value: string):CycleMapNoteValue
---@alias CycleMapGenerator fun(context: CycleMapContext, value: string):CycleMapFunction

---Map names in in the cycle to custom note events.
---
---By default, strings in cycles are interpreted as notes, and integer values as MIDI note
---values. Custom identifiers such as "bd" are undefined and will result into a rest, when
---they are not mapped explicitly.
---
---### examples:
---```lua
-----Using a static map table
---cycle("bd [bd, sn]"):map({
---  bd = "c4",
---  sn = "e4 #1 v0.2"
---})
---```
---```lua
-----Using a static map table with targets
---cycle("bd:1 <bd:5, bd:7>"):map({
---  -- instrument #1,5,7 are set additionally, as specified
---  bd = { key = "c4", volume = 0.5 },
---})
---```
---```lua
-----Using a dynamic map function
---cycle("4 5 4 <5 [4|6]>"):map(function(context, value)
---  -- emit a random note with 'value' as octave
---  return math.random(0, 11) + value * 12
---end)
---```
---```lua
-----Using a dynamic map function generator
---cycle("4 5 4 <4 [5|7]>"):map(function(init_context)
---  local notes = scale("c", "minor").notes
---  return function(context, value)
---    -- emit a 'cmin' note arp with 'value' as octave
---    local note = notes[math.imod(context.step, #notes)]
---    local octave = tonumber(value)
---    return { key = note + octave * 12 }
---  end
---end)
---```
---```lua
-----Using a dynamic map function to map values to chord degrees
---cycle("1 5 1 [6|7]"):map(function(init_context)
---  local cmin = scale("c", "minor")
---  return function(context, value)
---    return note(cmin:chord(tonumber(value)))
---  end
---end)
---```
---@param map { [string]: CycleMapNoteValue }|CycleMapFunction|CycleMapGenerator
---@return Cycle
---@nodiscard
function Cycle:map(map) end

---@alias CycleVarValue string|number|integer|boolean
---@alias CycleVarMap {[string]: CycleVarValue}
---@alias CycleVarFunction fun(context: CycleMapContext):CycleVarMap

---Assign variables to be used inside the main cycle script via `$name` notation
---
---By default any parameter you define will be available to the cycle as a variable
---but you can override and declare additional variables by passing in a table of them
---or passing a function that can returns such a table.
---
---### examples:
---```lua
-----Using a static table
---cycle("$a $a $b $a $b $b"):var({
---  a = "c a f e"
---  b = "e f a c"
---})
---```
---@param variables CycleVarMap|CycleVarFunction
---@return Cycle
---@nodiscard
function Cycle:var(variables) end

----------------------------------------------------------------------------------------------------

---Create a note sequence from a Tidal Cycles mini-notation string.
---
---`cycle` accepts a mini-notation as used by Tidal Cycles, with the following differences:
---* Stacks and random choices are valid without brackets (`a | b` is parsed as `[a | b]`)
---* `:` sets the instrument or remappable target instead of selecting samples but also
---  allows setting note attributes such as instrument/volume/pan/delay (e.g. `c4:v0.1:p0.5`)
---* In bjorklund expressions, operators *within* and on the *right side* are not supported
---  (e.g. `bd(<3 2>, 8)` and `bd(3, 8)*2` are *not* supported)
---
---[Tidal Cycles Reference](https://tidalcycles.org/docs/reference/mini_notation/)
---
---### examples:
--- ```lua
-----A chord sequence
---cycle("[c4, e4, g4] [e4, g4, b4] [g4, b4, d5] [b4, d5, f#5]")
---```
---```lua
-----Arpeggio pattern with variations
---cycle("<c4 e4 g4> <e4 g4> <g4 b4 d5> <b4 f5>")
---```
---```lua
-----Euclidean Rhythms
---cycle("c4(3,8) e4(5,8) g4(7,8)")
---```
---```lua
-----Map custom identifiers to notes
---cycle("bd(3,8)"):map({ bd = "c4 #1" })
--- ```
---@param input string
---@return Cycle
---@nodiscard
function cycle(input) end
