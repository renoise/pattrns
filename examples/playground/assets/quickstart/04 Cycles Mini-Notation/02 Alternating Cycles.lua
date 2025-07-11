-- Switching between different patterns
return pattern {
  unit = "1/4",
  event = cycle("[c4 e4 g4]|[d4 f4 a4]") -- Randomly select one of two chords
}

-- TRY THIS: Add more patterns with `|` like `[c4|c5 e4 g4]|[d4 f4|g5 a4]|[e4 g4 b4]`
-- TRY THIS: Try using `<>` instead of `[]` to select single alternating notes

-- See https://renoise.github.io/pattrns/guide/cycles.html
-- for more info about the Tidal Cycles mini-notation in pattrns.