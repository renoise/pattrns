# Notes, Chords & Scales

Note values, such as those specified in [event](./event.md), can be expressed and modified in various ways. Sometimes it's easier to generate notes programmatically using MIDI note numbers. Other times you may want to write down a chord in a more expressible form via a string. 

## Notes

### Note Numbers

Raw integer values like `48`, are interpreted as MIDI note numbers in the `note` function and `event` property. Valid MIDI notes are `0-127`.  

» `event = 48` *emit a single c4 note event*

### Note Tables

Instead of using a string, you can also specify notes via a Lua table with the following properties.

- `"key"` - REQUIRED - MIDI Note number such as `48` or a string, such as `"c4"`
- `"instrument"` OPTIONAL - Instrument/Sample/Patch number >= 0
- `"volume"`  - OPTIONAL - Volume number in range [0.0 - 1.0]
- `"panning"` - OPTIONAL - Panning factor in range [-1.0 - 1.0] where 0 is center
- `"delay"` - OPTIONAL - Delay factor in range [0.0 - 1.0]
- `'glide'` - OPTIONAL -  Glide factor in range [0-INF] as semitones per second


» `event = { key = 48, volume = 0.1 }` *a c4 with volume 0.1*

### Note Strings

Note strings such as `"c4"` are interpreted as `{KEY}{MOD}{OCT}` where MOD and OCT are optional.
Valid keys are `c,d,e,f,g,a,b`. Valid modifiers are `#` and `b`. Valid octaves are values `0-10`

» `event = "c4"` *emit a single c4 note*

Other note properties can be specified in the string notation as well.

- `'#'` instrument (integer >= 0)
- `'v'` volume (number in range [0-1])
- `'p'` panning (number in range [-1-1])
- `'d'` delay (number in range [0-1])
- `'g'` glide (number in range [0-INF])

» `event = { "f#4 #1 v0.2" }` *emit a f sharp for instrument 1 with volume 0.2*

### Note Chord Strings

To create a chords from a note string, append a `'` character to the key and specify a chord mode.

» `event = "d#4'maj"` *d#4 major chord*

See [chord Lua API](../API/chord.md#ChordName) for a list of all supported modes.

Just like regular notes, additional note properties can be added to the chord string as well.

» `event = "c4'69 #1 v0.5"` *instrument 1, volume 0.5*

### Note Objects

Note numbers, strings and tables, as described above, can be fed into a note object in the LuaAPI as well, which allows further transformation of the note.

This is especially handy for chords, but also can be more verbose than using note string attributes. 

» `event = note(48):volume(0.1)` *c4 note with volume of 0.1*

» `event = note({key = "c4"}):volume(0.2)` *c4 note with volume of 0.2*

» `event = note("c4'min"):transpose({12, 0, 0})` *C minor 1st inversion*

See [note Lua API](../API/note.md) for details.

### Note Chord Objects

Notes objects can also be created using the `chords` function.

» `event = chord(48, "major")` *c4 major notes*

This also allows the use of custom interval tables.

» `event = chord(48, {0,4,7})):volume(0.2)` *custom c4 major chord with volume 0.2*

See [chord Lua API](../API/chord.md) for details.

NB: The [sequence Lua API](../API/note.md) has a similar interface to modify notes within a sequence.

### Note Offs and Rests

To create rest values use Lua `nil` values, empty tables `{}`, empty strings `""`, or `"-"` strings.

To create off notes use the string `"off"` or `"~"`.

## Scales

To make working with chords and chord progressions and programming music in general easier, pattrns also has a simple scale API to create chords and notes from scales.


### Scale objects

Scale objects can be created from a note key and mode name, or custom intervals.

» `scale("c4", "minor").notes` *"c4", "d", "d#4", "f4", "g4", "g#4" "a#4"*

» `scale("c4", {0,2,3,5,7,8,10}).notes` *same as above*

#### Common Scales

See [scale Lua API](../API/scale.md#ScaleMode) for a list of all supported modes.

#### Custom Scales

Custom scales can be created by using an interval table with numbers from `0-11` in ascending order.

» `scale("c4", {0,3,5,7}).notes` *"c4", "d#4", "f4", "g4", "a4"*


### Scale Chords

The scale's `chord` function allows to generate chords from the scale's intervals.

```lua
local cmin = scale("c4", "minor")
return pattern {
  event = sequence(
    note(cmin:chord("i", 4)), --> note{48, 51, 55, 58}
    note(cmin:chord(5)):transpose({12, 0, 0}), --> Gm 1st inversion
  )
}
```

---

See [scale Lua API](../API/scale.md) for more information about scale objects.

