-- Create a pattern that alternates between notes
return pattern {
  unit = "1/8",         -- Eighth note timing grid
  pulse = {1, 0, 1, 1}, -- Play-rest-play-play pattern
  event = {"c4", "d4"}  -- Alternates between c4 and d4
}

-- TRY THIS: Change pattern to `{1, 1, 0, 1}` for a different rhythm
-- TRY THIS: Add more notes to emit like `{"c4", "d4", "e4", "g4"}`
