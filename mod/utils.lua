-- Utility functions for the Factorio mod

-- Format a number to a maximum of 5 decimal places
-- Rounds to 5 decimal places to avoid floating point precision issues
local function format_number(num)
  if type(num) ~= "number" then
    return num
  end

  -- Round to 5 decimal places
  local multiplier = 10^5
  local rounded = math.floor(num * multiplier + 0.5) / multiplier

  return rounded
end

-- Recursively format all numbers in a table to max 5 decimal places
local function format_numbers_in_table(tbl)
  local result = {}
  for key, value in pairs(tbl) do
    if type(value) == "number" then
      result[key] = format_number(value)
    elseif type(value) == "table" then
      result[key] = format_numbers_in_table(value)
    else
      result[key] = value
    end
  end
  return result
end

return {
  format_number = format_number,
  format_numbers_in_table = format_numbers_in_table
}
