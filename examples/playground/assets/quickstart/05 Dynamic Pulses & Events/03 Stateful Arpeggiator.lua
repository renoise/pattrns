-- Create patterns that remember previous states
return pattern {
  unit = "1/8",
  event = function(init_context)
    local notes = {"c4", "e4", "g4", "b4"}
    local index = 1 -- local state
    return function(context)
      index = math.imod(index + 1, #notes) -- Cycle through notes
      return notes[index]
    end
  end
}

-- TRY THIS: Add direction changes:
--   `if index >= #notes or index <= 1 then direction = direction * -1 end`
-- TRY THIS: Generate notes from a scale: 
--   `local notes = scale("C4", "major").notes`

-- See https://renoise.github.io/pattrns/extras/generators.html for more info 
-- about generators and local states.