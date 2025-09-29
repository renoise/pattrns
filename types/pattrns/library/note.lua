---@meta
error("Do not try to execute this file. It's just a type definition file.")
---
---Part of the pattrns crate: Defines LuaLS annotations for the pattrns Note class.
---

----------------------------------------------------------------------------------------------------

---@class NoteTable
---@field key string|number Note key & octave string (or MIDI note number as setter)
---@field instrument number? Instrument/Sample/Patch >= 0
---@field volume number? Volume in range [0.0 - 1.0]
---@field panning number? Panning factor in range [-1.0 - 1.0] where 0 is center
---@field delay number? Delay factor in range [0.0 - 1.0]
local NoteTable = {}

----------------------------------------------------------------------------------------------------

---@class Note
---@field notes NoteTable[]
local Note = {}

---Transpose the note with the specified step or steps.
---
---Values outside of the valid key range (0 - 127) will be clamped.
---
---### examples:
---```lua
---note("c4"):transpose(12)
---note("c'maj"):transpose(5)
---note("c'maj"):transpose({0, 0, -12})
---```
---@param step integer|integer[]
---@return Note
---@nodiscard
function Note:transpose(step) end

---Multiply the note's volume attribute with the specified factor or factors.
---
---Values outside of the valid volume range (0 - 1) will be clamped.
---
---### examples:
---```lua
---note({"c4 0.5", "g4"}):amplify(0.5)
---note("c'maj 0.5"):amplify({2.0, 1.0, 0.3})
---```
---@param factor number|number[] amplify factor > 0
---@return Note
---@nodiscard
function Note:amplify(factor) end

---Set the note's volume attribute to the specified value or values.
---
---### examples:
---```lua
---note({"c4", "g4"}):volume(0.5)
---note("c'maj"):volume(0.5)
---note("c'maj"):volume({0.1, 0.2, 0.3})
---```
---@param volume number|number[] new volume in range [0 - 1]
---@return Note
---@nodiscard
function Note:volume(volume) end

---Set the note's instrument attribute to the specified value or values.
---@param instrument number|number[]
---@return Note
---@nodiscard
function Note:instrument(instrument) end

---Set the note's panning attribute to the specified value or values.
---@param panning number|number[]
---@return Note
---@nodiscard
function Note:panning(panning) end

---Set the note's delay attribute to the specified value or values.
---@param delay number|number[]
---@return Note
---@nodiscard
function Note:delay(delay) end

---Set the note's glide attribute to the specified value or values.
---@param glide number|number[]
---@return Note
---@nodiscard
function Note:glide(glide) end

----------------------------------------------------------------------------------------------------

---@alias NoteValue Note|NoteTable|string|number|nil

--- Create a new monophonic or polyphonic note (a chord) from a number value,
--- a note string, chord string or array of note values.
---
--- In note strings the following prefixes are used to specify optional note
--- attributes:
---```md
--- -'#' -> instrument (integer > 0)
--- -'g' -> glide (number in range [0-INF] - glide step duration)
--- -'v' -> volume (number in range [0-1] - zero to full volume)
--- -'p' -> panning (number in range [-1-1] - full left to full right)
--- -'d' -> delay (number in range [0-1] - delay in units)
---```
---
---### examples:
--- ```lua
--- note(48) --> middle C
--- note("c4") --> middle C
--- note("c4 #2 v0.5 d0.3") --> middle C with additional properties
--- note({key="c4", volume=0.5}) --> middle C with volume 0.5
--- note("c4'maj v0.7") --> C4 major chord with volume 0.7
--- note("c4", "e4 v0.5", "off") --> custom chord with a c4, e4 and 'off' note
--- ```
---@param ... NoteValue
---@return Note
---@nodiscard
---@overload fun(table: NoteValue[]): Note
---@overload fun(...: NoteValue): Note
function note(...) end

---Convert a note string or note table to a raw MIDI note number in range 0-127
---or 0xFE for nil or empty note strings
---or 0xFF for note offs
---### Examples:
---```lua
---note_number("c4") --> 48
---note_number(note("c4")) --> 48
---note_number("-") --> 0xFE
---note_number("off") --> 0xFF
---note_number("xyz") --> error
---```
---@param note NoteValue
---@return integer
function note_number(note) end
