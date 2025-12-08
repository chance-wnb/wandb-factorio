-- Event handler for when a player builds/places an entity
script.on_event(defines.events.on_built_entity, function(event)
  local entity = event.entity
  local player = game.players[event.player_index]

  if entity and player then
    player.print("Item placed:" .. entity.name)
  end

end)
