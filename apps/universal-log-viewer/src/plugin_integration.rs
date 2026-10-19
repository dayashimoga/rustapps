#![forbid(unsafe_code)]
//! Plugin Host Integration for Universal Log Viewer
//!
//! Exposes extensible parser, highlighter, and sink plugins via `shared-plugins`.

use shared_plugins::{Plugin, PluginCapability, PluginError, PluginHost, PluginManifest};
use std::sync::Arc;
use std::time::Duration;

/// Built-in JSON flattener plugin
pub struct JsonFlattenerPlugin {
    manifest: PluginManifest,
}

impl Default for JsonFlattenerPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonFlattenerPlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest {
                id: "builtin.json-flattener".to_string(),
                name: "JSON Log Flattener".to_string(),
                version: "1.0.0".to_string(),
                author: "Universal Log Viewer Team".to_string(),
                capabilities: vec![PluginCapability::LogParser],
            },
        }
    }
}

impl Plugin for JsonFlattenerPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn init(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    fn execute(&self, input: &str) -> Result<String, PluginError> {
        let val: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| PluginError::ExecutionFailed(format!("Invalid JSON: {}", e)))?;

        if let serde_json::Value::Object(map) = val {
            let mut parts = Vec::new();
            for (k, v) in map {
                parts.push(format!("{}: {}", k, v));
            }
            Ok(parts.join(" | "))
        } else {
            Ok(input.to_string())
        }
    }

    fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

/// Built-in HTML / ANSI syntax highlighter plugin for log severities
pub struct SeverityHighlighterPlugin {
    manifest: PluginManifest,
}

impl Default for SeverityHighlighterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl SeverityHighlighterPlugin {
    pub fn new() -> Self {
        Self {
            manifest: PluginManifest {
                id: "builtin.severity-highlighter".to_string(),
                name: "Severity Syntax Highlighter".to_string(),
                version: "1.0.0".to_string(),
                author: "Universal Log Viewer Team".to_string(),
                capabilities: vec![PluginCapability::SyntaxHighlighter],
            },
        }
    }
}

impl Plugin for SeverityHighlighterPlugin {
    fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    fn init(&mut self) -> Result<(), PluginError> {
        Ok(())
    }

    fn execute(&self, input: &str) -> Result<String, PluginError> {
        let mut result = input.to_string();
        if result.contains("ERROR") || result.contains("FATAL") {
            result = format!("<span class=\"log-sev-error\">{}</span>", result);
        } else if result.contains("WARN") {
            result = format!("<span class=\"log-sev-warn\">{}</span>", result);
        } else if result.contains("INFO") {
            result = format!("<span class=\"log-sev-info\">{}</span>", result);
        } else if result.contains("DEBUG") || result.contains("TRACE") {
            result = format!("<span class=\"log-sev-debug\">{}</span>", result);
        }
        Ok(result)
    }

    fn shutdown(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

/// Thread-safe log viewer plugin manager wrapper
#[derive(Clone)]
pub struct LogViewerPluginManager {
    host: Arc<PluginHost>,
    registered_manifests: Arc<std::sync::RwLock<Vec<PluginManifest>>>,
}

impl Default for LogViewerPluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LogViewerPluginManager {
    /// Create and initialize manager with built-in plugins
    pub fn new() -> Self {
        let host = Arc::new(PluginHost::new(Duration::from_millis(2000)));
        let manifests = Arc::new(std::sync::RwLock::new(Vec::new()));

        let mgr = Self {
            host,
            registered_manifests: manifests,
        };

        // Register default plugins
        let p1 = Box::new(JsonFlattenerPlugin::new());
        let _ = mgr.register_plugin(p1);

        let p2 = Box::new(SeverityHighlighterPlugin::new());
        let _ = mgr.register_plugin(p2);

        mgr
    }

    /// Register a plugin instance
    pub fn register_plugin(&self, plugin: Box<dyn Plugin>) -> Result<(), PluginError> {
        let manifest = plugin.manifest().clone();
        self.host.register(plugin)?;

        if let Ok(mut lock) = self.registered_manifests.write() {
            if !lock.iter().any(|m| m.id == manifest.id) {
                lock.push(manifest);
            }
        }

        Ok(())
    }

    /// Execute a plugin by ID
    pub async fn execute(&self, id: &str, input: &str) -> Result<String, PluginError> {
        self.host.execute(id, input).await
    }

    /// List manifests of all active plugins
    pub fn list_plugins(&self) -> Vec<PluginManifest> {
        self.registered_manifests
            .read()
            .map(|l| l.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_manager_builtins() {
        let mgr = LogViewerPluginManager::new();
        let plugins = mgr.list_plugins();
        assert_eq!(plugins.len(), 2);
        assert!(plugins.iter().any(|p| p.id == "builtin.json-flattener"));
        assert!(plugins
            .iter()
            .any(|p| p.id == "builtin.severity-highlighter"));

        // Test JSON flattener
        let json_input = r#"{"user":"admin","action":"login"}"#;
        let flattened = mgr
            .execute("builtin.json-flattener", json_input)
            .await
            .unwrap();
        assert!(flattened.contains("user: \"admin\""));
        assert!(flattened.contains("action: \"login\""));

        // Test syntax highlighter
        let log_input = "2026-10-19 [ERROR] Database crashed";
        let highlighted = mgr
            .execute("builtin.severity-highlighter", log_input)
            .await
            .unwrap();
        assert!(highlighted.contains("<span class=\"log-sev-error\">"));
    }
}
