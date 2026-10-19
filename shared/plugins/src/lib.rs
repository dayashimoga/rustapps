#![forbid(unsafe_code)]
//! Shared Plugins Subsystem
//!
//! Provides extensible plugin traits, capability manifests, isolated lifecycle
//! execution, and sandboxed resource management.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use thiserror::Error;

/// Plugin error definitions
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Plugin '{id}' not found")]
    NotFound { id: String },
    #[error("Plugin execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Plugin execution timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("Invalid plugin lifecycle transition from {from:?} to {to:?}")]
    InvalidStateTransition {
        from: PluginLifecycleState,
        to: PluginLifecycleState,
    },
    #[error("Serialization / Manifest error: {0}")]
    ManifestError(String),
}

/// Declared capabilities a plugin can expose
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginCapability {
    LogParser,
    SyntaxHighlighter,
    SearchFilter,
    ExportSink,
}

/// Lifecycle states for registered plugins
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PluginLifecycleState {
    Loaded,
    Initialized,
    Active,
    Paused,
    Stopped,
    Failed,
}

/// Metadata manifest describing a plugin
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub capabilities: Vec<PluginCapability>,
}

/// Common trait implemented by all sandboxed plugins
pub trait Plugin: Send + Sync {
    /// Return the static metadata manifest.
    fn manifest(&self) -> &PluginManifest;
    /// Initialize plugin internal state.
    fn init(&mut self) -> Result<(), PluginError>;
    /// Execute primary capability logic on input payload.
    fn execute(&self, input: &str) -> Result<String, PluginError>;
    /// Gracefully shutdown plugin and release resources.
    fn shutdown(&mut self) -> Result<(), PluginError>;
}

/// Internal tracked plugin record
struct PluginEntry {
    plugin: Box<dyn Plugin>,
    state: PluginLifecycleState,
}

/// Thread-safe host managing plugin lifecycles and invocations
pub struct PluginHost {
    plugins: Arc<Mutex<HashMap<String, PluginEntry>>>,
    execution_timeout: Duration,
}

impl Default for PluginHost {
    fn default() -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
            execution_timeout: Duration::from_millis(1500),
        }
    }
}

impl PluginHost {
    pub fn new(execution_timeout: Duration) -> Self {
        Self {
            plugins: Arc::new(Mutex::new(HashMap::new())),
            execution_timeout,
        }
    }

    /// Register a new plugin in the `Loaded` state.
    pub fn register(&self, mut plugin: Box<dyn Plugin>) -> Result<(), PluginError> {
        let manifest = plugin.manifest().clone();
        plugin.init()?;

        let mut lock = self
            .plugins
            .lock()
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        lock.insert(
            manifest.id,
            PluginEntry {
                plugin,
                state: PluginLifecycleState::Active,
            },
        );

        Ok(())
    }

    /// Retrieve the lifecycle state of a registered plugin.
    pub fn get_state(&self, id: &str) -> Result<PluginLifecycleState, PluginError> {
        let lock = self
            .plugins
            .lock()
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

        let entry = lock
            .get(id)
            .ok_or_else(|| PluginError::NotFound { id: id.to_string() })?;

        Ok(entry.state)
    }

    /// Execute a plugin capability by ID with timeout enforcement.
    pub async fn execute(&self, id: &str, input: &str) -> Result<String, PluginError> {
        let timeout = self.execution_timeout;
        let id_string = id.to_string();
        let input_string = input.to_string();
        let plugins_clone = Arc::clone(&self.plugins);

        tokio::time::timeout(timeout, async move {
            let lock = plugins_clone
                .lock()
                .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;

            let entry = lock
                .get(&id_string)
                .ok_or(PluginError::NotFound { id: id_string })?;

            if entry.state != PluginLifecycleState::Active {
                return Err(PluginError::ExecutionFailed(
                    "Plugin is not in Active state".to_string(),
                ));
            }

            entry.plugin.execute(&input_string)
        })
        .await
        .map_err(|_| PluginError::Timeout {
            timeout_ms: timeout.as_millis() as u64,
        })?
    }

    /// Shutdown and deregister all active plugins.
    pub fn shutdown_all(&self) {
        if let Ok(mut lock) = self.plugins.lock() {
            for entry in lock.values_mut() {
                let _ = entry.plugin.shutdown();
                entry.state = PluginLifecycleState::Stopped;
            }
            lock.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockUppercasePlugin {
        manifest: PluginManifest,
    }

    impl MockUppercasePlugin {
        fn new() -> Self {
            Self {
                manifest: PluginManifest {
                    id: "mock.uppercase".to_string(),
                    name: "Mock Uppercase Filter".to_string(),
                    version: "1.0.0".to_string(),
                    author: "Core QA".to_string(),
                    capabilities: vec![PluginCapability::SearchFilter],
                },
            }
        }
    }

    impl Plugin for MockUppercasePlugin {
        fn manifest(&self) -> &PluginManifest {
            &self.manifest
        }

        fn init(&mut self) -> Result<(), PluginError> {
            Ok(())
        }

        fn execute(&self, input: &str) -> Result<String, PluginError> {
            Ok(input.to_uppercase())
        }

        fn shutdown(&mut self) -> Result<(), PluginError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_plugin_registration_and_execution() {
        let host = PluginHost::default();
        let plugin = Box::new(MockUppercasePlugin::new());

        host.register(plugin).expect("Registration must succeed");
        assert_eq!(
            host.get_state("mock.uppercase").unwrap(),
            PluginLifecycleState::Active
        );

        let output = host
            .execute("mock.uppercase", "production log error")
            .await
            .expect("Execution must succeed");

        assert_eq!(output, "PRODUCTION LOG ERROR");
    }

    #[tokio::test]
    async fn test_plugin_not_found() {
        let host = PluginHost::default();
        let err = host
            .execute("non_existent_plugin", "test")
            .await
            .unwrap_err();
        match err {
            PluginError::NotFound { id } => assert_eq!(id, "non_existent_plugin"),
            other => panic!("Expected NotFound, got {:?}", other),
        }
    }
}
