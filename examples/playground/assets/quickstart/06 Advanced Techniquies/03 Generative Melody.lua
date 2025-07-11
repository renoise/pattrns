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

-- TRY THIS: Add occasional jumps: 
--   `if math.random() < 0.1 then ...` (allow larger intervals)
-- TRY THIS: Change directions based on contour: 
--   add direction variable that occasionally flips
-- TRY THIS: Change scale to "mixolydian" or some other scale

-- See https://renoise.github.io/pattrns/extras/randomization.html for more info 
-- about randimization.

-- See https://renoise.github.io/pattrns/extras/generators.html for more info 
-- about generators and local states.