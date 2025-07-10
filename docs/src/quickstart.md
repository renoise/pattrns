# Quickstart

This guide will take you from basic patterns to advanced rhythmic sequencing through practical examples. Each example builds on previous concepts and includes comments to help you understand how things work.

NOTE: pattrns uses [Lua](https://www.lua.org/) as a scripting language. If you're not familiar with Lua, don't worry. Lua is very easy to pick up and fortunately there are great tutorials out there, such as [this one](https://w3schools.tech/tutorial/lua/index).

## Online Playground

All the examples in this quickstart are available in the [Online Playground](https://pattrns.renoise.com) as well, where you can run and modify them directly in your browser — no installation required.


## Table of Contents
- [Basic Patterns](#basic-patterns)
- [Rhythm Variations](#rhythm-variations)
- [Notes and Scales](#notes-and-scales)
- [Cycles Mini-Notation](#cycles-mini-notation)
- [Dynamic Pulses & Events](#dynamic-pulses--events)
- [Advanced Techniques](#advanced-techniques)


## Basic Patterns

### Quarter Note Pulse

```lua
-- The most basic rhythm: steady quarter notes
return pattern {
  unit = "1/4", -- Quarter note timing grid
  event = "c4"  -- Play middle C on each pulse
}
```
- TRY THIS: Change unit to `"1/8"` for eighth notes
- TRY THIS: Replace `"c4"` with `"e4"` or `"g4"` for different notes

This creates a steady pulse playing C4 on every quarter note. The `unit` parameter sets the timing grid, while `event` defines what note to play.

### Alternating Notes

```lua
-- Create a pattern that alternates between notes
return pattern {
  unit = "1/8",         -- Eighth note timing grid
  pulse = {1, 0, 1, 1}, -- Play-rest-play-play pattern
  event = {"c4", "d4"}  -- Alternates between C4 and D4
}
```
- TRY THIS: Change pattern to `{1, 1, 0, 1}` for a different rhythm
- TRY THIS: Add more note events like `{"c4", "d4", "e4", "g4"}`

Here, `pulse` controls when notes play (1) or rest (0). The `event` parameter cycles through the provided notes when a step triggers.

### Note properties

```lua
return pattern {
  unit = "1/8",
  event = { "c4 v0.2", "off d0.5", "g4 v0.8" }
}
```
- TRY THIS: Play specific instruments with # such as `c4 #8`
- TRY THIS: Add delays to some of the notes

Note events can be expressed in different ways. Use properties such as `v` = volume [0 - 1] `p` = panning [-1 - 1] `d` = delay [0 - 1] `#` = instrument to modify notes.

See [Notes & Scales](./guide/notes&scales.md) for more info out how to express note events.


## Rhythm Variations

### Subdivided Pulses

```lua
-- Pattern with mixed note lengths
return pattern {
  unit = "1/4",
  pulse = {1, {1, 1, 1, 1}}, -- One quarter note, then four sixteenth notes
  event = {"c4", "c5", "d4", "e4", "g4"} -- C4 (quarter), C5, d4, e4, g4 (sixteenth)
}
```
- TRY THIS: Try more complex subdivisions like {{1, 1}, {1, {1, 1}}}
- TRY THIS: Change the unit to "1/8" to make everything faster

Nested arrays in the pulse create subdivisions, allowing for more complex rhythms within the basic unit.

### Triplet Resolution

```lua
-- Create swing or triplet feel
return pattern {
  unit = "1/8",
  resolution = 2/3, -- Triplet feel (3 notes in space of 2)
  event = {"c4 v0.3", "e4 v0.5", "g4 v0.8"} -- v specifies volume, d delay, p panning
}
```
- TRY THIS: Change resolution to `"5/4"` for a different swing feel
- TRY THIS: Add values such as `d0.2` between 0 and 1 to delay specific notes

The `resolution` parameter modifies the timing grid, enabling triplet feels, swing rhythms, and polyrhythms.

### Euclidean Rhythms

```lua
-- Distributes notes evenly across steps (common in many music traditions)
return pattern {
  unit = "1/16",
  pulse = pulse.euclidean(3, 8), -- 3 hits spread over 8 steps
  event = "c4" -- Basic note
}
```
- TRY THIS: Try different combinations like `(5, 8)` or `(7, 16, -2)`
- TRY THIS: Use `pulse = pulse.euclidean(3, 8) + pulse.euclidean(5, 8)` to chain different patterns

Euclidean patterns distribute a number of notes evenly across steps, creating naturally pleasing rhythmic patterns found in many musical traditions. 

The [Pulse API](./API/pulse.md) contains various tools to programatically create patterns.


## Notes and Scales

### Basic Note Stacks

```lua
-- Simple chord by stacking notes
return pattern {
  unit = "1/1",
  event = {{"c4", "e4", "g4"}, "c4"}  -- C major chord followed by a single C4
}
```
- TRY THIS: Try different chord combinations like `{"d4", "f4", "a4"}` for D minor
- TRY THIS: Add `v` values to create dynamics: `{"c4 v0.8", "e4 v0.6", "g4 v0.4"}`

A table of notes allows emitting multiple notes at once, creating chords and harmonies.

### Chord Notation

```lua
-- Using chord notation shortcuts
return pattern {
  unit = "1/1",
  event = {
    "c4'M",   -- C major using ' chord notation
    "d4'm",   -- D minor
    "g4'dom7" -- G dominant 7th
  }
}
```
- TRY THIS: Use other chord modes like `'m5`, `'+`, or `'dim`
- TRY THIS: Add inversions with `note("c4'M"):transpose({12, 0, 0})`

Chord notation provides a quick way to specify common chord types without listing individual notes.

### Working with Scales

```lua
-- Advanced chord and scale operations
return pattern {
  unit = "1/1",
  event = {
    chord("c4", "major"),         -- C major via the chord function
    chord("c4", {0, 4, 7}),       -- C major via custom intervals
    scale("c", "major"):chord(1), -- C major from 1st degree of C major scale
    scale("c", "major"):chord(5)  -- G major from 5th degree of C major scale
  }
}
```
- TRY THIS: Use other scales like `"minor"`, `"dorian"`, or `"pentatonic"`
- TRY THIS: Try different chord degrees: `scale("c", "major"):chord(2)` for D minor

The `scale()` function allows creating chords from scale degrees, enabling an easy way to e.g. create chord progressions.

See available modes and scales at the [API docs](./API/scale.md). See [Notes and Scales](./guide/notes&scales.md) for more ways to create and manipulate notes and chords.

## Tidal Cycles Mini-Notation

### Basic Cycle

```lua
-- Using tidal cycles notation for concise patterns
return pattern {
  unit = "1/4", -- Emit a cycle every beat
  event = cycle("c4 e4 g4") -- C major arpeggio
}
```

```lua
-- The simplified notation emits a cycle **per bar**
return cycle("c4 e4 g4") 
```

Tidal Cycles' mini-notation provides a concise way to express patterns.

### Alternating Cycles

```lua
-- Switching between different patterns
return pattern {
  unit = "1/4",
  event = cycle("[c4 e4 g4]|[d4 f4 a4]") -- Randomly select one of two chords
}
```
- TRY THIS: Add more patterns with `|` like `[c4|c5 e4 g4]|[d4 f4|g5 a4]|[e4 g4 b4]`
- TRY THIS: Try simultaneous notes with square brackets `[c4 e4]`

The `|` operator in cycles randomly selects different patterns.

### Euclidean Rhythms in Cycles

```lua
-- Euclidean patterns in tidal cycles notation
return cycle("c4(3,8) e4(5,8) g4(7,8)")  -- Different Euclidean rhythms
```
- TRY THIS: Combine with alternation: `c4(3,8)|e4(5,8)`
- TRY THIS: Change the numbers for different distributions

Tidal Cycles mini-notation also supports Euclidean patterns with the `(n,k)` notation, where `n` is the number of notes and `k` is the number of steps.

See [Cycles Guide](./guide/cycles.md) for more example and info about Tidal Cycles in pattrns.


## Dynamic Pulses & Events

### Random Note Selection

```lua
-- Randomly select notes from a list
local notes = {"c4", "d4", "e4", "g4"}
return pattern {
  unit = "1/8",
  event = function(context)
    return notes[math.random(#notes)] -- Pick random note from array
  end
}
```
- TRY THIS: Use notes from a specific scale with `local notes = scale("c4", "major").notes`
- TRY THIS: Add amplitude variation with `note(some_note):amplify(0.5 + math.random() * 0.5)`

Using functions for emitters enables dynamic behavior, like randomly selecting notes from a predefined set.

### Probability-Based Emitting

```lua
-- Emit notes with certain probability
return pattern {
  unit = "1/8",
  pulse = {1, 1, 1, 1},  -- Regular pattern
  event = function(context)
    if math.random() < 0.3 then  -- 30% chance to emit
      return "c4"
    end
  end
}
```
- TRY THIS: Vary probability by step position: `if math.random() < (context.step % 4) / 4 then`
- TRY THIS: Higher probability on downbeats: `if math.random() < ((context.pulse_step - 1) % 2 == 0 and 0.8 or 0.2) then`

This pattern uses a gate function to filter notes with a 30% probability, creating a sparse, probabilistic pulse. Gates are evaluated after patterns but before emission, giving you control over which triggered notes actually play.

### Stateful Arpeggiator

```lua
-- Create patterns that remember previous states
return pattern {
  unit = "1/8",
  event = function(init_context)
    local notes = {"c4", "e4", "g4", "b4"}
    local index = 1
    return function(context)
      local note = notes[index]
      index = math.imod(index + 1, #notes) -- Cycle through notes
      return note
    end
  end
}
```
- TRY THIS: Add direction changes: `if index >= #notes or index <= 1 then direction = direction * -1 end`
- TRY THIS: Change notes based on time: `notes = scale("C4", {"major","minor"}[math.floor(context.pulse_time) % 2 + 1]).notes`

This example demonstrates stateful emitters that remember their position between calls, enabling sequences and other time-dependent behaviors.


## Advanced Techniques

### Conditional Gate

```lua
-- Filter which notes actually play using gates
return pattern {
  unit = "1/8",
  pulse = {1, 0.1, 1, 0.5, 1, 0.2, 1, 0.1}, -- probability values
  gate = function(context)
    -- always play on even-numbered step values
    return (context.pulse_step - 1) % 2 == 0 or
      -- else use pulse values as probablities
      context.pulse_value >= math.random() 
  end,
  event = "c4"
}
```
- TRY THIS: Create a threshold gate: `context.pulse_value > 0.5`
- TRY THIS: Only play when a specific MIDI note is held: `context.trigger.notes[1].key == "C4"`

Gates filter which triggered notes actually play, adding another layer of control to your patterns.

### Dynamic Cycles

```lua
-- Identifiers in cycles can be dynamically mapped to something else
local s = scale("C4", "minor")
return pattern {
  unit = "1/4",
  event = cycle("I III V VII"):map(function(context, value)
    -- value here is a single roman number from the cycle above
    local degree = value
    -- apply value as roman number chord degree
    return s:chord(degree)
  end)
}
```
- TRY THIS: Change scale to `"major", "dorian", or "pentatonic minor"`
- TRY THIS: Add parameters: `parameter.enum("scale", "minor", {"major", "minor", "pentatonic"})`

This example uses musical scale knowledge to generate chord progressions based on scale degrees.

See [Parameters](./guide/parameters.md) on how to add template parameters to patterns.

### Generative Melody with Constraints

```lua
-- Create melodies that follow musical rules
return pattern {
  unit = "1/8",
  event = function(init_context)
    local pentatonic = scale("c4", "pentatonic minor").notes
    local last_note = 1
    return function(context)
      local next_note = math.random(#pentatonic)
      -- Prefer steps of 1 or 2 scale degrees (smoother melodies)
      while math.abs(next_note - last_note) > 2 do
        next_note = math.random(#pentatonic)
      end
      last_note = next_note
      return pentatonic[next_note]
    end
  end
}
```
- TRY THIS: Add occasional jumps: `if math.random() < 0.1 then ...` (allow larger intervals)
- TRY THIS: Change directions based on contour: add direction variable that occasionally flips

This generates melodies that follow musical constraints, like preferring small intervals for more natural-sounding melodies.

## Further Resources

- [Guide](./guide/)
- [Advanced Topics](./extras/)
- [Guided Examples](./examples/)
- [API Reference](./API/)

Remember to experiment by modifying these examples! The best way to learn is by tweaking parameters and seeing what happens.
