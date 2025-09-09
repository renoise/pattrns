# note
<!-- toc -->
# Global<a name="Global"></a>  

---  
## Functions
### note(...[`NoteValue`](#NoteValue))<a name="note"></a>
`->`[`Note`](../API/note.md#Note)  

>  Create a new monophonic or polyphonic note (a chord) from a number value,
>  a note string, chord string or array of note values.
> 
>  In note strings the following prefixes are used to specify optional note
>  attributes:
> ```md
>  -'#' -> instrument (integer > 0)
>  -'v' -> volume (number in range [0-1] - zero to full volume)
>  -'p' -> panning (number in range [-1-1] - full left to full right)
>  -'d' -> delay (number in range [0-1] - delay in time base units)
>  -'g' -> glide (number in range [0-INF] - semitones per second)
> ```
> 
> #### examples:
>  ```lua
>  note(48) --> middle C
>  note("c4") --> middle C
>  note("c4 #2 v0.5 d0.3") --> middle C with additional properties
>  note({key="c4", volume=0.5}) --> middle C with volume 0.5
>  note("c4'maj v0.7") --> C4 major chord with volume 0.7
>  note("c4", "e4 v0.5", "off") --> custom chord with a c4, e4 and 'off' note
>  ```
### note_number(note : [`NoteValue`](#NoteValue))<a name="note_number"></a>
`->`[`integer`](../API/builtins/integer.md)  

> Convert a note string or note table to a raw MIDI note number in range 0-127
> or 0xFE for nil or empty note strings
> or 0xFF for note offs
> ### Examples:
> ```lua
> note_number("c4") --> 48
> note_number(note("c4")) --> 48
> note_number("-") --> 0xFE
> note_number("off") --> 0xFF
> note_number("xyz") --> error
> ```  



---  
## Aliases  
### NoteValue<a name="NoteValue"></a>
[`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md) | [`Note`](../API/note.md#Note) | [`NoteTable`](../API/note.md#NoteTable) | [`nil`](../API/builtins/nil.md)  
  
  



# Note<a name="Note"></a>  

---  
## Properties
### notes : [`NoteTable`](../API/note.md#NoteTable)[]<a name="notes"></a>
  

---  
## Functions
### transpose([*self*](../API/builtins/self.md), step : [`integer`](../API/builtins/integer.md) | [`integer`](../API/builtins/integer.md)[])<a name="transpose"></a>
`->`[`Note`](../API/note.md#Note)  

> Transpose the note with the specified step or steps.
> 
> Values outside of the valid key range (0 - 127) will be clamped.
> 
> #### examples:
> ```lua
> note("c4"):transpose(12)
> note("c'maj"):transpose(5)
> note("c'maj"):transpose({0, 0, -12})
> ```
### amplify([*self*](../API/builtins/self.md), factor : [`number`](../API/builtins/number.md) | [`number`](../API/builtins/number.md)[])<a name="amplify"></a>
`->`[`Note`](../API/note.md#Note)  

> Multiply the note's volume attribute with the specified factor or factors.
> 
> Values outside of the valid volume range (0 - 1) will be clamped.
> 
> #### examples:
> ```lua
> note({"c4 0.5", "g4"}):amplify(0.5)
> note("c'maj 0.5"):amplify({2.0, 1.0, 0.3})
> ```
### volume([*self*](../API/builtins/self.md), volume : [`number`](../API/builtins/number.md) | [`number`](../API/builtins/number.md)[])<a name="volume"></a>
`->`[`Note`](../API/note.md#Note)  

> Set the note's volume attribute to the specified value or values.
> 
> #### examples:
> ```lua
> note({"c4", "g4"}):volume(0.5)
> note("c'maj"):volume(0.5)
> note("c'maj"):volume({0.1, 0.2, 0.3})
> ```
### instrument([*self*](../API/builtins/self.md), instrument : [`number`](../API/builtins/number.md) | [`number`](../API/builtins/number.md)[])<a name="instrument"></a>
`->`[`Note`](../API/note.md#Note)  

> Set the note's instrument attribute to the specified value or values.
### panning([*self*](../API/builtins/self.md), panning : [`number`](../API/builtins/number.md) | [`number`](../API/builtins/number.md)[])<a name="panning"></a>
`->`[`Note`](../API/note.md#Note)  

> Set the note's panning attribute to the specified value or values.
### delay([*self*](../API/builtins/self.md), delay : [`number`](../API/builtins/number.md) | [`number`](../API/builtins/number.md)[])<a name="delay"></a>
`->`[`Note`](../API/note.md#Note)  

> Set the note's delay attribute to the specified value or values.
### glide([*self*](../API/builtins/self.md), glide : [`number`](../API/builtins/number.md) | [`number`](../API/builtins/number.md)[])<a name="glide"></a>
`->`[`Note`](../API/note.md#Note)  

> Set the note's glide attribute to the specified value or values.  



# NoteTable<a name="NoteTable"></a>  

---  
## Properties
### key : [`string`](../API/builtins/string.md) | [`number`](../API/builtins/number.md)<a name="key"></a>
> Note key & octave string (or MIDI note number as setter)

### instrument : [`number`](../API/builtins/number.md)[`?`](../API/builtins/nil.md)<a name="instrument"></a>
> Instrument/Sample/Patch >= 0

### volume : [`number`](../API/builtins/number.md)[`?`](../API/builtins/nil.md)<a name="volume"></a>
> Volume in range [0.0 - 1.0]

### panning : [`number`](../API/builtins/number.md)[`?`](../API/builtins/nil.md)<a name="panning"></a>
> Panning factor in range [-1.0 - 1.0] where 0 is center

### delay : [`number`](../API/builtins/number.md)[`?`](../API/builtins/nil.md)<a name="delay"></a>
> Delay factor in range [0.0 - 1.0]

  



