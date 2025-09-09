---@meta
error("Do not try to execute this file. It's just a type definition file.")
---
---Part of the pattrns crate: Defines LuaLS annotations for the pattrns Sequence class.
---

----------------------------------------------------------------------------------------------------

---@class Sequence
---@field notes NoteTable[][]
local Sequence = {}

---Transpose all note's key values with the specified step value or values.
---
---Values outside of the valid key range (0 - 127) will be clamped.
---
---### examples:
---```lua
---sequence("c4", "d#5"):transpose(12)
---sequence(note("c'maj"), note("c'maj")):transpose({0, 5})
---```
---@param step integer|integer[]
---@return Sequence
---@nodiscard
function Sequence:transpose(step) end

---Multiply all note's volume values with the specified factor or factors.
---
---Values outside of the valid volume range (0 - 1) will be clamped.
---
---### examples:
---```lua
---sequence({"c4 0.5", "g4"}):amplify(0.5)
---sequence("c'maj 0.5"):amplify({2.0, 1.0, 0.3})
---```
---@param factor number|number[]
---@return Sequence
---@nodiscard
function Sequence:amplify(factor) end

---Set the instrument attribute of all notes to the specified value or values.
---@param instrument number|number[]
---@return Note
---@nodiscard
function Sequence:instrument(instrument) end

---Set the volume attribute of all notes to the specified value or values.
---
---### examples:
---```lua
---sequence({"c4", "g4"}):volume(0.5)
---sequence("c'maj"):volume(0.5)
---sequence("c'maj"):volume({0.1, 0.2, 0.3})
---```
---@param volume number|number[]
---@return Sequence
---@nodiscard
function Sequence:volume(volume) end

---Set the panning attribute of all notes to the specified value or values.
---@param panning number|number[]
---@return Note
---@nodiscard
function Sequence:panning(panning) end

---Set the delay attribute of all notes to the specified value or values.
---@param delay number|number[]
---@return Sequence
---@nodiscard
function Sequence:delay(delay) end

---Set the glide attribute of all notes to the specified value or values.
---@param glide number|number[]
---@return Sequence
---@nodiscard
function Sequence:glide(glide) end

----------------------------------------------------------------------------------------------------

---Create a sequence from an array of note values or note value varargs.
---
---Using `sequence` instead of a raw `{}` table can be useful to ease transforming the note
---content and to explicitly pass a sequence of e.g. single notes to the event emitter.
---
---### examples:
---```lua
----- sequence of C4, C5 and an empty note
---sequence(48, "c5", {})
----- sequence of a +5 transposed C4 and G4 major chord
---sequence("c4'maj", "g4'maj"):transpose(5)
--- ```
---@param ... NoteValue|Note
---@return Sequence
---@nodiscard
---@overload fun(table: (NoteValue|Note)[]): Sequence
---@overload fun(...: (NoteValue|Note)): Sequence
function sequence(...) end
