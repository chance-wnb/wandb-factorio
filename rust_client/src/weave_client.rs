use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// Configuration for the Weave client
#[derive(Debug, Clone)]
pub struct WeaveConfig {
    pub entity: String,
    pub project: String,
    pub base_url: String,
    pub api_key: String,
    pub binary_path: PathBuf,
    pub socket_path: PathBuf,
}

impl WeaveConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, String> {
        let entity = env::var("WEAVE_ENTITY")
            .map_err(|_| "WEAVE_ENTITY environment variable not set")?;
        let project = env::var("WEAVE_PROJECT")
            .map_err(|_| "WEAVE_PROJECT environment variable not set")?;
        let base_url = env::var("WEAVE_BASE_URL")
            .unwrap_or_else(|_| "https://trace.wandb.ai".to_string());
        let api_key = env::var("WANDB_API_KEY")
            .map_err(|_| "WANDB_API_KEY environment variable not set")?;

        let binary_path = env::var("WEAVE_BINARY_PATH")
            .map_err(|_| "WEAVE_BINARY_PATH environment variable not set")?;

        let binary_path = PathBuf::from(binary_path).join("weave-sender");

        if !binary_path.exists() {
            return Err(format!("Weave binary not found at {:?}", binary_path));
        }

        let socket_path = PathBuf::from(format!(
            "/tmp/weave-sender-factorio-{}.sock",
            std::process::id()
        ));

        Ok(Self {
            entity,
            project,
            base_url,
            api_key,
            binary_path,
            socket_path,
        })
    }

    /// Get the project_id in the format "entity/project"
    pub fn project_id(&self) -> String {
        format!("{}/{}", self.entity, self.project)
    }
}

/// JSON-RPC request structure
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    id: i32,
    method: String,
    params: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_reply: Option<bool>,
}

/// JSON-RPC response structure
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    id: i32,
    #[serde(default)]
    result: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

/// Auth params for the weave-sender
#[derive(Debug, Serialize)]
struct AuthParams {
    username: String,
    password: String,
}

/// Init params for the weave-sender
#[derive(Debug, Serialize)]
struct InitParams {
    server_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    auth: Option<AuthParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, String>>,
}

/// Enqueue params for the weave-sender
#[derive(Debug, Serialize)]
struct EnqueueParams {
    items: Vec<EnqueueItem>,
}

#[derive(Debug, Serialize)]
struct EnqueueItem {
    #[serde(rename = "type")]
    item_type: String, // "start" or "end"
    payload: serde_json::Value,
}

/// StartedCallSchemaForInsert as per Weave trace server interface
#[derive(Debug, Serialize)]
pub struct StartedCallSchemaForInsert {
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub op_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    pub started_at: DateTime<Utc>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub inputs: HashMap<String, serde_json::Value>,
}

/// EndedCallSchemaForInsert as per Weave trace server interface
#[derive(Debug, Serialize)]
pub struct EndedCallSchemaForInsert {
    pub project_id: String,
    pub id: String,
    pub ended_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    pub summary: HashMap<String, serde_json::Value>,
}

/// CallStartReq wrapper
#[derive(Debug, Serialize)]
struct CallStartReq {
    start: StartedCallSchemaForInsert,
}

/// CallEndReq wrapper
#[derive(Debug, Serialize)]
struct CallEndReq {
    end: EndedCallSchemaForInsert,
}

/// Weave client that communicates with the Go weave-sender via Unix socket
pub struct WeaveClient {
    config: WeaveConfig,
    process: Arc<Mutex<Option<Child>>>,
    connection: Arc<Mutex<Option<UnixStream>>>,
    request_id: Arc<Mutex<i32>>,
}

impl WeaveClient {
    /// Create a new Weave client
    pub fn new(config: WeaveConfig) -> Self {
        Self {
            config,
            process: Arc::new(Mutex::new(None)),
            connection: Arc::new(Mutex::new(None)),
            request_id: Arc::new(Mutex::new(1)),
        }
    }

    /// Initialize the client by starting the Go sender and establishing connection
    pub async fn init(&self) -> Result<(), String> {
        // Start the weave-sender process
        self.start_sender_process().await?;

        // Wait for socket to be available
        self.wait_for_socket().await?;

        // Connect to the socket
        self.connect_to_socket().await?;

        // Initialize the sender
        self.send_init().await?;

        println!("✅ Weave client initialized successfully");
        Ok(())
    }

    /// Start the weave-sender process
    async fn start_sender_process(&self) -> Result<(), String> {
        let mut process_guard = self.process.lock().await;

        if process_guard.is_some() {
            return Ok(());
        }

        println!(
            "🚀 Starting weave-sender: {:?}",
            self.config.binary_path
        );

        let child = Command::new(&self.config.binary_path)
            .arg("-socket")
            .arg(&self.config.socket_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| format!("Failed to start weave-sender: {}", e))?;

        *process_guard = Some(child);
        Ok(())
    }

    /// Wait for the socket file to be created
    async fn wait_for_socket(&self) -> Result<(), String> {
        for _ in 0..50 {
            if self.config.socket_path.exists() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err("Weave-sender socket did not become available".to_string())
    }

    /// Connect to the Unix socket
    async fn connect_to_socket(&self) -> Result<(), String> {
        let stream = UnixStream::connect(&self.config.socket_path)
            .map_err(|e| format!("Failed to connect to socket: {}", e))?;

        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| format!("Failed to set read timeout: {}", e))?;

        let mut conn_guard = self.connection.lock().await;
        *conn_guard = Some(stream);

        Ok(())
    }

    /// Send init command to the weave-sender
    async fn send_init(&self) -> Result<(), String> {
        let params = InitParams {
            server_url: self.config.base_url.clone(),
            auth: Some(AuthParams {
                username: String::new(), // W&B uses API key as username
                password: self.config.api_key.clone(), // Empty password
            }),
            headers: None,
        };

        let response = self
            .send_request("init", serde_json::to_value(params).unwrap(), false)
            .await?;

        if let Some(error) = response.error {
            return Err(format!("Init failed: {}", error.message));
        }

        Ok(())
    }

    /// Send a JSON-RPC request and wait for response
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
        no_reply: bool,
    ) -> Result<JsonRpcResponse, String> {
        let mut id_guard = self.request_id.lock().await;
        let id = *id_guard;
        *id_guard += 1;
        drop(id_guard);

        let request = JsonRpcRequest {
            id,
            method: method.to_string(),
            params,
            no_reply: if no_reply { Some(true) } else { None },
        };

        let mut request_json = serde_json::to_string(&request)
            .map_err(|e| format!("Failed to serialize request: {}", e))?;
        request_json.push('\n');

        // Send request
        let mut conn_guard = self.connection.lock().await;
        let conn = conn_guard
            .as_mut()
            .ok_or_else(|| "Not connected".to_string())?;

        conn.write_all(request_json.as_bytes())
            .map_err(|e| format!("Failed to write request: {}", e))?;

        if no_reply {
            // Don't wait for response
            return Ok(JsonRpcResponse {
                id,
                result: None,
                error: None,
            });
        }

        // Read response
        let mut reader = BufReader::new(conn.try_clone().unwrap());
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(response)
    }

    /// Start a call (send to queue with fire-and-forget)
    pub async fn start_call(
        &self,
        start: StartedCallSchemaForInsert,
    ) -> Result<(), String> {
        let req = CallStartReq { start };
        let payload = serde_json::to_value(req)
            .map_err(|e| format!("Failed to serialize start call: {}", e))?;

        let params = EnqueueParams {
            items: vec![EnqueueItem {
                item_type: "start".to_string(),
                payload,
            }],
        };

        // Fire-and-forget for performance
        self.send_request("enqueue", serde_json::to_value(params).unwrap(), true)
            .await?;

        Ok(())
    }

    /// End a call (send to queue with fire-and-forget)
    pub async fn end_call(&self, end: EndedCallSchemaForInsert) -> Result<(), String> {
        let req = CallEndReq { end };
        let payload = serde_json::to_value(req)
            .map_err(|e| format!("Failed to serialize end call: {}", e))?;

        let params = EnqueueParams {
            items: vec![EnqueueItem {
                item_type: "end".to_string(),
                payload,
            }],
        };

        // Fire-and-forget for performance
        self.send_request("enqueue", serde_json::to_value(params).unwrap(), true)
            .await?;

        Ok(())
    }

    /// Flush all pending items
    pub async fn flush(&self) -> Result<(), String> {
        let response = self
            .send_request("flush", serde_json::json!({}), false)
            .await?;

        if let Some(error) = response.error {
            return Err(format!("Flush failed: {}", error.message));
        }

        Ok(())
    }

    /// Wait for queue to be empty
    pub async fn wait_queue_empty(&self) -> Result<(), String> {
        let response = self
            .send_request("wait_queue_empty", serde_json::json!({}), false)
            .await?;

        if let Some(error) = response.error {
            return Err(format!("Wait queue empty failed: {}", error.message));
        }

        Ok(())
    }

    /// Wait for all in-flight requests to complete
    pub async fn wait_idle(&self) -> Result<(), String> {
        // Check if connection exists before trying to send
        {
            let conn_guard = self.connection.lock().await;
            if conn_guard.is_none() {
                return Err("Connection already closed".to_string());
            }
        }

        let response = self
            .send_request("wait_idle", serde_json::json!({}), false)
            .await?;

        if let Some(error) = response.error {
            return Err(format!("Wait idle failed: {}", error.message));
        }

        Ok(())
    }

    /// Get statistics from the sender
    pub async fn stats(&self) -> Result<serde_json::Value, String> {
        let response = self
            .send_request("stats", serde_json::json!({}), false)
            .await?;

        if let Some(error) = response.error {
            return Err(format!("Stats failed: {}", error.message));
        }

        Ok(response.result.unwrap_or(serde_json::json!({})))
    }

    /// Shutdown the weave-sender
    pub async fn shutdown(&self) -> Result<(), String> {
        // Check if process is still running before attempting communication
        let process_alive = {
            let mut process_guard = self.process.lock().await;
            if let Some(child) = process_guard.as_mut() {
                // Check if process has already exited
                match child.try_wait() {
                    Ok(Some(status)) => {
                        println!("🔷 Weave-sender already exited with status: {}", status);
                        false
                    }
                    Ok(None) => true, // Still running
                    Err(e) => {
                        eprintln!("⚠️  Failed to check process status: {}", e);
                        false
                    }
                }
            } else {
                false
            }
        };

        // Only try to send shutdown command if process is alive
        if process_alive {
            if let Err(e) = self
                .send_request("shutdown", serde_json::json!({}), false)
                .await
            {
                eprintln!("⚠️  Shutdown command failed (process may have died): {}", e);
            }
        }

        // Close connection
        let mut conn_guard = self.connection.lock().await;
        *conn_guard = None;
        drop(conn_guard);

        // Wait for process to exit
        let mut process_guard = self.process.lock().await;
        if let Some(mut child) = process_guard.take() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Already exited, nothing to do
                }
                Ok(None) => {
                    // Still running, wait for it
                    if let Err(e) = child.wait() {
                        eprintln!("⚠️  Failed to wait for weave-sender process: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Failed to check process status: {}", e);
                }
            }
        }

        // Clean up socket file
        if self.config.socket_path.exists() {
            if let Err(e) = std::fs::remove_file(&self.config.socket_path) {
                eprintln!("⚠️  Failed to remove socket file: {}", e);
            }
        }

        println!("✅ Weave client shutdown complete");
        Ok(())
    }
}

impl Drop for WeaveClient {
    fn drop(&mut self) {
        // Try to clean up, but don't panic if it fails
        // Note: This is a blocking drop, which is not ideal for async code
        // In production, you should call shutdown() explicitly before dropping
        if let Some(mut child) = self.process.try_lock().ok().and_then(|mut g| g.take()) {
            let _ = child.kill();
        }

        if self.config.socket_path.exists() {
            let _ = std::fs::remove_file(&self.config.socket_path);
        }
    }
}
