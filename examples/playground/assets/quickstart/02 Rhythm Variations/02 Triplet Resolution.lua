-- Create swing or triplet feel
return pattern {
  unit = "1/8",
  resolution = 2/3, -- Triplet feel (3 notes in space of 2)
  event = {"c4", "e4", "g4"}
}

-- TRY THIS: Change resolution to "5/4" for a different swing feel
-- TRY THIS: Add note properties such as `d0.2` to delay a single note

-- See https://renoise.github.io/pattrns/guide/timebase.html for more info
-- about time units and resolution.