-- Distributes notes evenly across steps (common in many music traditions)
return pattern {
  unit = "1/16",
  pulse = pulse.euclidean(3, 8, 0),  -- 3 hits spread over 8 steps without offset
  event = "c4"
}

-- TRY THIS: Try different combinations like (5, 8, -2) or (7, 16)
-- TRY THIS: Use `pulse = pulse.euclidean(3, 8) + pulse.euclidean(5, 8)`
--   to chain different patterns

-- See https://renoise.github.io/pattrns/API/pulse.html for more info about
-- the pulse helper library. 