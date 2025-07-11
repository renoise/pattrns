-- Pattern with mixed note lengths
return pattern {
  unit = "1/4",
  pulse = {1, {1, 1, 1, 1}}, -- One quarter note, then four sixteenth notes
  event = {"c4", "c5", "e4", "g4", "d4"} -- C4 (quarter), c5, e4, g4, d4 (sixteenth notes)
}

-- TRY THIS: Try more complex subdivisions like `{{1, 1}, {1, {1, 1}}}`
-- TRY THIS: Change the unit to "1/8" to make everything faster

-- See https://renoise.github.io/pattrns/guide/pulse.html for more info about
-- pulses and pulse subdivisions.