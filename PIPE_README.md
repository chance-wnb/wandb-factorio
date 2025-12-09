# Factorio Data Bridge: Technical Core

This document explains the core technology stack of the project: **Named Pipe (FIFO)**.

## 1. Technical Core

This project leverages **Named Pipe (FIFO)** technology to bypass the sandbox restrictions of Factorio, establishing a real-time data channel from inside the game to external AI.

### Named Pipe (FIFO)
**What is it?**
A "virtual file" that exists in the file system. Writers write into it, and readers read from it. Data flows directly in the kernel memory without touching the disk.

**Why use it?**
*   **Factorio Limitations**: The game's Mod sandbox (**Lua**) prohibits the use of TCP/UDP Sockets and loading C extensions.
*   **The Only Way Out**: The only efficient I/O operation permitted for Mods is "writing to files".
*   **The Hack**: We trick the Mod into thinking it is writing to a regular file (`events.pipe`), but in reality, it is writing to a Pipe we pre-created. This achieves **real-time data escape from the closed sandbox to an external process**.

Once the data reaches the external Python Daemon, we can use standard Python networking libraries (Weave SDK) to send it to the cloud.

---

## 2. Execution Guide (How to Run)

Establish the entire pipeline in just three steps.

### Step 1: Setup Pipe
The pipe must be created before the game starts; otherwise, Factorio will create a regular text file.

```bash
# macOS/Linux
mkfifo "$HOME/Library/Application Support/factorio/script-output/events.pipe"
```

### Step 2: Start Daemon
Start the Python script. It will block and vigilantly monitor this pipe.
*Note: This script has a built-in auto-cleanup mechanism. It automatically rebuilds the pipe on startup and deletes it on exit, requiring no manual maintenance.*

```bash
# Dependencies required: pip install weave wandb
python3 python_daemon/daemon.py
```

### Step 3: Start Game
1.  Install the Mod (put `factorio_mod` into the `mods` directory).
2.  Launch Factorio -> **New Game**.
3.  The Mod code `helpers.write_file("events.pipe", ...)` begins execution.
4.  Data flow is instantly established.

---

## 3. Cleanup & Lifecycle

**Automatic Mode**:
The Python Daemon has built-in `atexit` cleanup logic.
*   **On Start**: Automatically deletes old Pipe and creates a new one.
*   **On Exit**: Automatically deletes the Pipe.
*   **You don't need to do anything.**

**Manual Mode (If script crashes)**:
If the Python process is forcibly killed (kill -9) and fails to clean up, you can manually reset it with the following commands:

```bash
rm "$HOME/Library/Application Support/factorio/script-output/events.pipe"
mkfifo "$HOME/Library/Application Support/factorio/script-output/events.pipe"
```
