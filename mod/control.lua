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
  local player_force = game.forces["player"]
  local nauvis = game.surfaces["nauvis"]

  if player_force and nauvis then
    local item_stats = player_force.get_item_production_statistics(nauvis)
    local fluid_stats = player_force.get_fluid_production_statistics(nauvis)

    -- Build stats data structure
    local stats_data = {
      cycle = math.floor(event.tick / 120),
      tick = event.tick,
      products_production = {},
      materials_consumption = {}
    }

    -- Collect item production rates (items per minute)
    for item_name, _ in pairs(item_stats.input_counts) do
      local rate = item_stats.get_flow_count{
        name = item_name,
        category = "input",
        precision_index = defines.flow_precision_index.one_minute
      }
      if rate > 0 then
        stats_data.products_production[item_name] = rate
      end
    end

    -- Collect item consumption rates (items per minute)
    for item_name, _ in pairs(item_stats.output_counts) do
      local rate = item_stats.get_flow_count{
        name = item_name,
        category = "output",
        precision_index = defines.flow_precision_index.one_minute
      }
      if rate > 0 then
        stats_data.materials_consumption[item_name] = rate
      end
    end

    -- Collect fluid production rates (per minute)
    for fluid_name, _ in pairs(fluid_stats.input_counts) do
      local rate = fluid_stats.get_flow_count{
        name = fluid_name,
        category = "input",
        precision_index = defines.flow_precision_index.one_minute
      }
      if rate > 0 then
        stats_data.products_production[fluid_name] = rate
      end
    end

    -- Collect fluid consumption rates (per minute)
    for fluid_name, _ in pairs(fluid_stats.output_counts) do
      local rate = fluid_stats.get_flow_count{
        name = fluid_name,
        category = "output",
        precision_index = defines.flow_precision_index.one_minute
      }
      if rate > 0 then
        stats_data.materials_consumption[fluid_name] = rate
      end
    end

    -- Convert to JSON and write to named pipe
    local json_str = helpers.table_to_json(stats_data)
    helpers.write_file("events.pipe", json_str .. "\n", true)
  end
end)
