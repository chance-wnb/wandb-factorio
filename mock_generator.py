import json
import time
import random
import uuid
import os

# 配置
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

# Session ID 格式: level_tick_random (配合 Chance 的定义)
LEVEL_NAME = "nauvis"
INIT_TICK = int(time.time()) 
SESSION_ID = f"{LEVEL_NAME}_{INIT_TICK}_{random.randint(100000, 999999)}"

EVENT_TYPES = ["on_built_entity", "on_player_mined_entity", "on_research_started", "on_research_finished"]
ENTITIES = ["assembling-machine-1", "inserter", "transport-belt", "iron-chest", "stone-furnace"]
TECHS = ["automation", "logistics", "steel-processing", "electronics"]
RECIPES = ["iron-plate", "copper-cable", "electronic-circuit", "inserter"]
ITEMS = ["iron-plate", "copper-plate", "electronic-circuit", "coal", "stone"]
FLUIDS = ["water", "crude-oil", "petroleum-gas"]

def ensure_pipe():
    if not os.path.exists(PIPE_PATH):
        try:
            os.mkfifo(PIPE_PATH)
            print(f"Created named pipe at {PIPE_PATH}")
        except OSError as e:
            print(f"Failed to create pipe: {e}")

# 生成 session_init 事件 (这是 Chance 的 Rust Client 识别新 Run 的关键)
def generate_session_init():
    return {
        "type": "session_init",
        "session_id": SESSION_ID,
        "tick": 0,
        "level_name": LEVEL_NAME
    }

def generate_event(tick):
    event_name = random.choice(EVENT_TYPES + ["on_player_crafted_item"])
    
    payload = {
        "type": "event", 
        "session_id": SESSION_ID,
        "tick": tick,
        "event_name": event_name,
        "player_index": 1,
    }
    
    if "research" in event_name:
        payload["tech_name"] = random.choice(TECHS)
        payload["level"] = random.randint(1, 5)
        if event_name == "on_research_finished":
            payload["duration"] = round(random.uniform(60, 600), 2)
    elif event_name == "on_player_crafted_item":
        item = random.choice(RECIPES)
        payload["item_name"] = item
        payload["count"] = random.randint(1, 5)
        payload["recipe"] = item
    else:
        payload["entity"] = random.choice(ENTITIES)
        payload["position"] = {"x": round(random.uniform(0, 100), 1), "y": round(random.uniform(0, 100), 1)}
        
    return payload

def format_number(num):
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
        "type": "stats", # 适配 Chance 的 'stats' 类型
        "session_id": SESSION_ID,
        "cycle": tick // 120,
        "tick": tick,
        
        # 扩展字段：玩家状态 (目前 Rust Client 可能还没用，但以后会有用)
        "player": {
            "position": {"x": round(random.uniform(0, 100), 1), "y": round(random.uniform(0, 100), 1)},
            "surface": "nauvis",
            "health": 250.0
        },
        "screenshot_path": f"scans/{SESSION_ID}/tick_{tick}.jpg",

        "products_production": products_prod,
        "materials_consumption": materials_cons
    }

def main():
    print(f"Starting Mock Generator... Session: {SESSION_ID}")
    print(f"Writing to: {PIPE_PATH}")
    
    ensure_pipe()
    
    try:
        with open(PIPE_PATH, "w") as f:
            # 1. 开局先发 session_init
            init_data = generate_session_init()
            f.write(json.dumps(init_data) + "\n")
            f.flush()
            print(f"[Init] Session Initialized")
            
            tick = 0
            while True:
                time.sleep(0.5) 
                tick += 30 
                
                # 2. 随机生成 Event
                if random.random() < 0.2: 
                    event_data = generate_event(tick)
                    f.write(json.dumps(event_data) + "\n")
                    f.flush()
                    print(f"[Event] {event_data['event_name']}")

                # 3. 定期生成 Status
                if tick % 120 == 0:
                    status_data = generate_status(tick)
                    f.write(json.dumps(status_data) + "\n")
                    f.flush()
                    print(f"[Stats] Tick {tick}")
                    
    except KeyboardInterrupt:
        print("\nStopping Mock Generator.")
    except BrokenPipeError:
        print("\nPipe closed by reader.")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    main()
