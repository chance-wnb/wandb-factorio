import json
import time
import random
import uuid
import os

# Configuration
POSSIBLE_PATHS = [
    os.path.expanduser("~/Library/Application Support/factorio/script-output/events.pipe"),
    "/tmp/events.pipe"
]

PIPE_PATH = POSSIBLE_PATHS[0]
for p in POSSIBLE_PATHS:
    try:
        if os.path.exists(os.path.dirname(p)):
            PIPE_PATH = p
            break
    except:
        continue

SESSION_ID = f"nauvis_{int(time.time())}_{random.randint(100000, 999999)}"

EVENT_TYPES = ["on_built_entity", "on_player_mined_entity", "on_research_started", "on_research_finished"]
ENTITIES = ["assembling-machine-1", "inserter", "transport-belt", "iron-chest", "stone-furnace"]
TECHS = ["automation", "logistics", "steel-processing", "electronics"]
ITEMS = ["iron-plate", "copper-plate", "electronic-circuit", "coal", "stone"]
FLUIDS = ["water", "crude-oil", "petroleum-gas"]

def ensure_pipe():
    if not os.path.exists(PIPE_PATH):
        try:
            os.mkfifo(PIPE_PATH)
            print(f"Created named pipe at {PIPE_PATH}")
        except OSError as e:
            print(f"Failed to create pipe: {e}")
            
def generate_event(tick):
    event_name = random.choice(EVENT_TYPES)
    
    payload = {
        "stream": "event", 
        "session_id": SESSION_ID,
        "tick": tick,
        "event_name": event_name,
        "player_index": 1,
    }
    
    # Populate different fields based on event type
    if "research" in event_name:
        payload["tech_name"] = random.choice(TECHS)
        if event_name == "on_research_finished":
            payload["duration"] = round(random.uniform(60, 600), 2)  # Simulated research duration (seconds)
    else:
        # Build/Mine events
        payload["entity"] = random.choice(ENTITIES)
        payload["position"] = {"x": round(random.uniform(0, 100), 1), "y": round(random.uniform(0, 100), 1)}
        
    return payload

def format_number(num):
    # Return number rounded to 5 decimal places (consistent with Chance's utils.lua)
    return round(num, 5)

def generate_status(tick):
    products_prod = {}
    materials_cons = {}
    
    for item in ITEMS:
        if random.random() > 0.3:
            products_prod[item] = format_number(random.uniform(0, 200))
        if random.random() > 0.3:
            materials_cons[item] = format_number(random.uniform(0, 200))
            
    for fluid in FLUIDS:
         if random.random() > 0.5:
            products_prod[fluid] = format_number(random.uniform(0, 500))

    return {
        "session_id": SESSION_ID,
        "cycle": tick // 120,
        "tick": tick,
        "products_production": products_prod,
        "materials_consumption": materials_cons
    }

def main():
    print(f"Starting Mock Generator... Session: {SESSION_ID}")
    print(f"Writing to: {PIPE_PATH}")
    
    ensure_pipe()
    
    tick = 0
    try:
        with open(PIPE_PATH, "w") as f:
            while True:
                # 1. Simulate game tick
                time.sleep(0.5) 
                tick += 30 
                
                # 2. Randomly generate Event
                if random.random() < 0.2: 
                    event_data = generate_event(tick)
                    f.write(json.dumps(event_data) + "\n")
                    f.flush()
                    print(f"[Event] {event_data['event_name']}")

                # 3. Periodically generate Status
                if tick % 120 == 0:
                    status_data = generate_status(tick)
                    f.write(json.dumps(status_data) + "\n")
                    f.flush()
                    print(f"[Status] Tick {tick}")
                    
    except KeyboardInterrupt:
        print("\nStopping Mock Generator.")
    except BrokenPipeError:
        print("\nPipe closed by reader.")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    main()
