-- Filter which notes actually play using gates
return pattern {
  unit = "1/8",
  pulse = {1, 0.1, 1, 0.5, 1, 0.2, 1, 0.1}, -- probability values
  gate = function(context)
    -- always play on even-numbered step values
    return (context.pulse_step - 1) % 2 == 0 or
      -- else use pulse values as probabilities
      context.pulse_value >= math.random() 
  end,
  event = "c4"
}

-- TRY THIS: Create a threshold gate: `context.pulse_value > 0.5`
-- TRY THIS: Only play when a specific MIDI note is held: 
--   `context.trigger.notes[1].key == "C4"`

-- See https://renoise.github.io/pattrns/guide/gate.html for more info about
-- pattrns gate functions.