-- Weave Agent Logger
-- 
-- Architecture:
-- 1. collect_game_state(): Gathers data from the game (This is where Mod Design logic goes).
-- 2. on_tick(): Orchestrates the frequency and writes to the pipe (This is the named.pipe writing logic).

-- ============================================================================
-- SECTION 1: Game State Collection (Edit this to change WHAT data is sent)
-- ============================================================================
local function collect_game_state(event)
    local state = {
        tick = event.tick,
        timestamp = game.tick,
        surfaces = {},
        player = nil,
        production = nil
    }

    -- 1. Environment Data
    local surface = game.surfaces["nauvis"]
    if surface then
        state.surfaces["nauvis"] = {
            daytime = surface.daytime,
            wind_speed = surface.wind_speed,
        }
    end

    -- 2. Player Data (MVP: Inventory Count)
    -- TODO: Expand this to include real production statistics or other entities
    local player = game.players[1]
    if player and player.connected then
        state.player = {
            position = {x = player.position.x, y = player.position.y},
        }
        
        -- Current MVP: Using inventory count as a proxy for production
        local iron = player.get_item_count("iron-plate")
        local copper = player.get_item_count("copper-plate")

        state.production = {
            iron_plate = { produced = iron, consumed = 0 },
            copper_plate = { produced = copper, consumed = 0 }
        }
    end

    return state
end

-- ============================================================================
-- SECTION 2: Infrastructure (Do not edit unless changing IO behavior)
-- ============================================================================
script.on_event(defines.events.on_tick, function(event)
    -- Frequency Control: 1Hz (60 ticks)
    if event.tick % 60 ~= 0 then return end

    -- 1. Gather Data
    local data = collect_game_state(event)

    -- 2. Serialize (Factorio 2.0 API)
    local json_str = helpers.table_to_json(data)

    -- 3. Write to Pipe (Append mode is critical for named pipes)
    helpers.write_file("events.pipe", json_str .. "\n", true)
end)
