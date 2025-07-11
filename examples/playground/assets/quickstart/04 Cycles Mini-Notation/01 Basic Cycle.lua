-- Using tidal cycles notation for concise patterns
return pattern {
  unit = "1/4", -- Emit a cycle every beat
  event = cycle("c4 e4 g4") -- C major arpeggio
}

-- TRY THIS: The simplified notation (without return patterns) emits a cycle per bar
--   `return cycle("c4 e4 g4")`

-- See https://renoise.github.io/pattrns/guide/cycles.html
-- for more info about the Tidal Cycles mini-notation in pattrns.