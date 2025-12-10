use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::sync::{Arc, Mutex};
use std::thread;

/// Shared cache for pipe events that can be accessed by other parts of the application
#[derive(Clone)]
pub struct PipeCache {
    events: Arc<Mutex<VecDeque<String>>>,
}

impl PipeCache {
    /// Create a new PipeCache with specified capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            events: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
        }
    }

    /// Start the background reader thread
    pub fn start_reader(&self, pipe_path: String, log_path: Option<String>) {
        let events = self.events.clone();

        thread::spawn(move || {
            println!("Pipe reader thread started");
            println!("Reading from: {}", pipe_path);

            // Open log file if specified
            let mut log_file = log_path.as_ref().map(|path| {
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .expect("Failed to open log file")
            });

            // Open the pipe once and keep reading
            loop {
                match File::open(&pipe_path) {
                    Ok(pipe) => {
                        println!("Successfully opened pipe");
                        let mut reader = BufReader::new(pipe);
                        let mut line = String::new();

                        // Keep reading lines from the same pipe
                        loop {
                            line.clear();
                            match reader.read_line(&mut line) {
                                Ok(0) => {
                                    // EOF reached - writer closed the pipe
                                    // This is normal, just reopen
                                    thread::sleep(std::time::Duration::from_millis(100));
                                    break;
                                }
                                Ok(_) => {
                                    // Successfully read a line
                                    let trimmed = line.trim();
                                    if !trimmed.is_empty() {
                                        // Add to cache
                                        {
                                            let mut cache = events.lock().unwrap();
                                            cache.push_back(trimmed.to_string());

                                            // Remove old events if capacity exceeded
                                            if cache.len() > 10000 {
                                                cache.pop_front();
                                            }
                                        }

                                        // Write to log file if specified
                                        if let Some(ref mut log) = log_file {
                                            writeln!(log, "{}", trimmed).ok();
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Error reading line: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to open pipe: {}, retrying in 1 second...", e);
                        thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
            }
        });
    }

    /// Get all events in the cache (non-destructive read)
    pub fn get_all(&self) -> Vec<String> {
        self.events.lock().unwrap().iter().cloned().collect()
    }

    /// Get the last N events (non-destructive read)
    pub fn get_last_n(&self, n: usize) -> Vec<String> {
        let cache = self.events.lock().unwrap();
        cache.iter().rev().take(n).rev().cloned().collect()
    }

    /// Get the most recent event (non-destructive read)
    pub fn get_latest(&self) -> Option<String> {
        self.events.lock().unwrap().back().cloned()
    }

    /// Pop the oldest event (destructive read)
    pub fn pop_front(&self) -> Option<String> {
        self.events.lock().unwrap().pop_front()
    }

    /// Drain all events (destructive read)
    pub fn drain_all(&self) -> Vec<String> {
        let mut cache = self.events.lock().unwrap();
        cache.drain(..).collect()
    }

    /// Get the current number of cached events
    pub fn len(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.events.lock().unwrap().is_empty()
    }

    /// Filter events by a predicate (non-destructive read)
    pub fn filter<F>(&self, predicate: F) -> Vec<String>
    where
        F: Fn(&str) -> bool,
    {
        self.events
            .lock()
            .unwrap()
            .iter()
            .filter(|line| predicate(line))
            .cloned()
            .collect()
    }

    /// Find events containing a specific string
    pub fn find_containing(&self, search: &str) -> Vec<String> {
        self.filter(|line| line.contains(search))
    }
}
