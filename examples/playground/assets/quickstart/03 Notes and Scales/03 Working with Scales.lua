-- Advanced chord and scale operations
return pattern {
  unit = "1/1",
  event = {
    chord("c4", "major"),          -- C major via the chord function
    chord("c4", {0, 4, 7}),        -- C major via custom intervals
    scale("c", "major"):chord(1),  -- C major from 1st degree of C major scale
    scale("c", "major"):chord(5)   -- G major from 5th degree of C major scale
  }
}

-- TRY THIS: Use other scales like `"minor"`, `"dorian"`, or `"pentatonic"`
-- TRY THIS: Try different chord degrees: e.g. `scale("c", "major"):chord(2)` for d minor

-- See https://renoise.github.io/pattrns/API/scale.html for more info about scales