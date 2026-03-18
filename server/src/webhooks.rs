//! Webhook system — dispatch events to registered HTTP endpoints.
//!
//! Events: document.created, document.deleted, document.exported

use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// A registered webhook endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub id: String,
    pub url: String,
    pub events: Vec<String>,
    pub active: bool,
    pub created_at: String,
}

/// Webhook event payload sent to endpoints.
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct WebhookEvent {
    pub event: String,
    pub timestamp: String,
    pub data: serde_json::Value,
}

/// In-memory webhook registry.
pub struct WebhookRegistry {
    hooks: Mutex<Vec<Webhook>>,
}

impl WebhookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: Mutex::new(Vec::new()),
        }
    }

    /// Register a new webhook.
    pub fn register(&self, webhook: Webhook) {
        let mut hooks = self.hooks.lock().unwrap();
        hooks.push(webhook);
    }

    /// Remove a webhook by ID.
    pub fn remove(&self, id: &str) -> bool {
        let mut hooks = self.hooks.lock().unwrap();
        let len_before = hooks.len();
        hooks.retain(|h| h.id != id);
        hooks.len() < len_before
    }

    /// List all registered webhooks.
    pub fn list(&self) -> Vec<Webhook> {
        self.hooks.lock().unwrap().clone()
    }

    /// Dispatch an event to all matching webhooks.
    ///
    /// Delivery is async and fire-and-forget — failures are logged but don't block.
    #[allow(dead_code)]
    pub fn dispatch(&self, event_name: &str, data: serde_json::Value) {
        let hooks = self.hooks.lock().unwrap().clone();
        let matching: Vec<_> = hooks
            .into_iter()
            .filter(|h| h.active && h.events.iter().any(|e| e == event_name || e == "*"))
            .collect();

        if matching.is_empty() {
            return;
        }

        let event = WebhookEvent {
            event: event_name.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            data,
        };

        // Fire-and-forget: spawn a task for each delivery
        for hook in matching {
            let payload = serde_json::to_string(&event).unwrap_or_default();
            let url = hook.url.clone();
            tokio::spawn(async move {
                match reqwest::Client::new()
                    .post(&url)
                    .header("Content-Type", "application/json")
                    .header("User-Agent", "s1-server/1.0")
                    .body(payload)
                    .timeout(std::time::Duration::from_secs(10))
                    .send()
                    .await
                {
                    Ok(resp) => {
                        tracing::debug!("Webhook delivered to {}: {}", url, resp.status());
                    }
                    Err(e) => {
                        tracing::warn!("Webhook delivery failed for {}: {}", url, e);
                    }
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_list() {
        let reg = WebhookRegistry::new();
        assert_eq!(reg.list().len(), 0);

        reg.register(Webhook {
            id: "wh1".into(),
            url: "https://example.com/hook".into(),
            events: vec!["document.created".into()],
            active: true,
            created_at: "2026-03-19".into(),
        });

        assert_eq!(reg.list().len(), 1);
        assert_eq!(reg.list()[0].id, "wh1");
    }

    #[test]
    fn remove_webhook() {
        let reg = WebhookRegistry::new();
        reg.register(Webhook {
            id: "wh2".into(),
            url: "https://example.com/hook2".into(),
            events: vec!["*".into()],
            active: true,
            created_at: "2026-03-19".into(),
        });

        assert!(reg.remove("wh2"));
        assert_eq!(reg.list().len(), 0);
        assert!(!reg.remove("nonexistent"));
    }
}
