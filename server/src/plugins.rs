//! Plugin API endpoint system.
//!
//! Plugins can register custom REST endpoints under `/api/v1/plugins/{name}/`.
//! Each plugin provides an Axum Router that handles its own routes.
//!
//! # Example
//!
//! ```rust,ignore
//! use axum::{routing::get, Json, Router};
//! use s1_server::plugins::PluginRouter;
//!
//! fn my_plugin() -> PluginRouter {
//!     PluginRouter {
//!         name: "analytics".to_string(),
//!         router: Router::new()
//!             .route("/stats", get(|| async { Json(serde_json::json!({"views": 42})) })),
//!     }
//! }
//! ```
#![allow(dead_code)]

use axum::Router;
use std::sync::Arc;

use crate::routes::AppState;

/// A plugin's router registration.
pub struct PluginRouter {
    /// Plugin name (used as URL prefix: /api/v1/plugins/{name}/).
    pub name: String,
    /// The plugin's Axum router with its own routes.
    pub router: Router<Arc<AppState>>,
}

/// Registry of plugin routers.
pub struct PluginRegistry {
    plugins: Vec<PluginRouter>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Register a plugin router.
    pub fn register(&mut self, plugin: PluginRouter) {
        tracing::info!(
            "Plugin registered: {} → /api/v1/plugins/{}/",
            plugin.name,
            plugin.name
        );
        self.plugins.push(plugin);
    }

    /// Build the combined plugin router.
    ///
    /// Each plugin's routes are nested under `/plugins/{name}/`.
    pub fn build(self) -> Router<Arc<AppState>> {
        let mut router = Router::new();
        for plugin in self.plugins {
            router = router.nest(&format!("/plugins/{}", plugin.name), plugin.router);
        }
        router
    }

    /// Get the number of registered plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Get the names of all registered plugins.
    pub fn names(&self) -> Vec<&str> {
        self.plugins.iter().map(|p| p.name.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;

    #[test]
    fn register_and_count() {
        let mut registry = PluginRegistry::new();
        assert_eq!(registry.count(), 0);

        registry.register(PluginRouter {
            name: "test".to_string(),
            router: Router::new().route("/hello", get(|| async { "world" })),
        });

        assert_eq!(registry.count(), 1);
        assert_eq!(registry.names(), vec!["test"]);
    }
}
