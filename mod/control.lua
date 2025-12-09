-- Event handler for when a player builds/places an entity
script.on_event(defines.events.on_built_entity, function(event)
  local entity = event.entity
  local player = game.players[event.player_index]

  if entity and player then
    player.print("Item placed:" .. entity.name)
  end
end)

-- Periodic production/consumption rate dump (every 120 ticks = 2 seconds)
script.on_nth_tick(120, function(event)
  game.print("=== Production/Consumption Rates at tick " .. event.tick .. " ===")

  local player_force = game.forces["player"]
  local nauvis = game.surfaces["nauvis"]

  if player_force and nauvis then
    local item_stats = player_force.get_item_production_statistics(nauvis)
    local fluid_stats = player_force.get_fluid_production_statistics(nauvis)

    -- Dump item production rates (items per minute)
    game.print("Item Production Rate (per minute):")
    for item_name, _ in pairs(item_stats.input_counts) do
      local rate = item_stats.get_flow_count{
        name = item_name,
        category = "input",
        precision_index = defines.flow_precision_index.one_minute
      }
      if rate > 0 then
        game.print("  " .. item_name .. ": " .. string.format("%.2f", rate))
      end
    end

    -- Dump item consumption rates (items per minute)
    game.print("Item Consumption Rate (per minute):")
    for item_name, _ in pairs(item_stats.output_counts) do
      local rate = item_stats.get_flow_count{
        name = item_name,
        category = "output",
        precision_index = defines.flow_precision_index.one_minute
      }
      if rate > 0 then
        game.print("  " .. item_name .. ": " .. string.format("%.2f", rate))
      end
    end

    -- Dump fluid production rates (per minute)
    game.print("Fluid Production Rate (per minute):")
    for fluid_name, _ in pairs(fluid_stats.input_counts) do
      local rate = fluid_stats.get_flow_count{
        name = fluid_name,
        category = "input",
        precision_index = defines.flow_precision_index.one_minute
      }
      if rate > 0 then
        game.print("  " .. fluid_name .. ": " .. string.format("%.2f", rate))
      end
    end

    -- Dump fluid consumption rates (per minute)
    game.print("Fluid Consumption Rate (per minute):")
    for fluid_name, _ in pairs(fluid_stats.output_counts) do
      local rate = fluid_stats.get_flow_count{
        name = fluid_name,
        category = "output",
        precision_index = defines.flow_precision_index.one_minute
      }
      if rate > 0 then
        game.print("  " .. fluid_name .. ": " .. string.format("%.2f", rate))
      end
    end
  end

  game.print("==========================================")
end)
