---@meta
error("Do not try to execute this file. It's just a type definition file.")
---
---Part of the pattrns crate: Defines LuaLS annotations for the pattrns chord function.
---

----------------------------------------------------------------------------------------------------

---Available chords.
---@alias ChordName "major"|"major7"|"major9"|"major11"|"major13"|"minor"|"minor#5"|"minor6"|"minor69"|"minor7b5"|"minor7"|"minor7#5"|"minor7b9"|"minor7#9"|"minor9"|"minor11"|"minor13"|"minorMajor7"|"add9"|"add11"|"add13"|"dom7"|"dom9"|"dom11"|"dom13"|"7b5"|"7#5"|"7b9"|"five"|"six"|"sixNine"|"seven"|"nine"|"eleven"|"thirteen"|"augmented"|"diminished"|"diminished7"|"sus2"|"sus4"|"7sus2"|"7sus4"|"9sus2"|"9sus4"|string

---Create a new chord from the given key notes and a chord name or an array of custom intervals.
---
---NB: Chords also can also be defined via strings in function `note` and via the a scale's
---`chord` function. See examples below.
---
---Chord names also can be shortened by using the following synonyms:
---- "min" | "m" | "-" -> "minor"
---- "maj" | "M" | "^" -> "major"
---- "minMaj" | "mM" | "-^" -> "minMajor"
---- "+" | "aug" -> "augmented"
---- "o" | "dim" -> "diminished"
---- "5 -> "five"
---- "6 -> "six"
---- "69" -> "sixNine"
---- "7 -> "seven"
---- "9 -> "nine"
---- "11" -> "eleven"
---
---### examples:
---```lua
---chord("c4", "minor") --> {"c4", "d#4", "f4"}
---chord({key = 48, volume = 0.5}, "minor") --> {"c4 v0.5", "d#4 v0.5", "f4 v0.5"}
-----same as:
---chord("c4", {0, 4, 7})
---chord("c4 v0.5", {0, 4, 7})
-----or:
---note("c4'major")
---note("c4'major v0.5")
-----or:
---note(scale("c4", "major"):chord("i", 3))
---note(scale("c4", "major"):chord("i", 3)):volume(0.5)
---```
---@param key NoteValue e.g. "c4" or 48
---@param mode ChordName
---@return Note
---@nodiscard
---@overload fun(key: NoteValue, intervals: integer[]): Note
function chord(key, mode) end

---Return supported chord names.
---@return string[]
function chord_names() end
