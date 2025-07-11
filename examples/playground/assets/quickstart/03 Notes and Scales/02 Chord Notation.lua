-- Using chord notation shortcuts
return pattern {
  unit = "1/1",
  event = {
    "c4'M",   -- C major using ' chord notation
    "d4'm",   -- D minor
    "g4'dom7" -- G dominant 7th
  }
}

-- TRY THIS: Use other chord modes like `'m5`, `'+`, or `'dim`
-- TRY THIS: Add inversions with `note("c4'M"):transpose({12, 0, 0})`

-- See https://renoise.github.io/pattrns/API/chord.html for a list
-- of all available chord modes and related information.