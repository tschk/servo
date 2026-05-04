/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/. */

//! Soliloquy-specific bootstrap for integrating the browser optimization crate
//! into the in-tree Servo shell.

use std::collections::HashMap;

use log::info;
use servo::WebViewId;
use soliloquy_browser_optimizations::{
    cache::CachedResource,
    network::{PrefetchPriority, ResourceType},
    LruCache, PrefetchManager, TabResidencyManager,
};
use url::Url;

pub(crate) struct SoliloquyOptimizations {
    active_webview_id: Option<WebViewId>,
    bootstrap_url: Option<String>,
    webview_tabs: HashMap<WebViewId, u64>,
    webview_urls: HashMap<WebViewId, String>,
    residency: TabResidencyManager,
    prefetch: PrefetchManager,
    navigation_cache: LruCache<String>,
}

impl SoliloquyOptimizations {
    pub(crate) fn new() -> Self {
        SoliloquyOptimizations {
            active_webview_id: None,
            bootstrap_url: None,
            webview_tabs: HashMap::new(),
            webview_urls: HashMap::new(),
            residency: TabResidencyManager::new(),
            prefetch: PrefetchManager::new(),
            navigation_cache: LruCache::new(8 * 1024 * 1024),
        }
    }

    pub(crate) fn bootstrap(&mut self, initial_url: &Url) {
        let initial_url = initial_url.as_str().to_string();
        self.bootstrap_url = Some(initial_url.clone());
        self.queue_prefetches(initial_url.as_str(), PrefetchPriority::High);
        self.cache_navigation("bootstrap-url", &initial_url);
    }

    pub(crate) fn register_webview(&mut self, webview_id: WebViewId, url: Option<&Url>) {
        let url = url
            .map(|url| url.as_str().to_string())
            .or_else(|| self.bootstrap_url.clone())
            .unwrap_or_else(|| format!("servo:webview:{webview_id:?}"));

        let tab_id = self
            .webview_tabs
            .get(&webview_id)
            .copied()
            .unwrap_or_else(|| self.residency.register_tab(url.clone()));

        self.webview_tabs.insert(webview_id, tab_id);
        self.webview_urls.insert(webview_id, url.clone());
        let _ = self.residency.touch_tab(tab_id);
        self.queue_prefetches(url.as_str(), PrefetchPriority::High);
        self.cache_navigation(&format!("webview:{webview_id:?}:created"), &url);
    }

    pub(crate) fn activate_webview(&mut self, webview_id: WebViewId) {
        self.active_webview_id = Some(webview_id);
        if let Some(tab_id) = self.webview_tabs.get(&webview_id).copied() {
            let _ = self.residency.touch_tab(tab_id);
        }
    }

    pub(crate) fn record_navigation_request(&mut self, webview_id: WebViewId, url: &Url) {
        self.upsert_webview_url(webview_id, url, PrefetchPriority::High, "requested");
    }

    pub(crate) fn record_navigation_committed(&mut self, webview_id: WebViewId, url: &Url) {
        self.upsert_webview_url(webview_id, url, PrefetchPriority::Medium, "committed");
    }

    pub(crate) fn close_webview(&mut self, webview_id: WebViewId) {
        if let Some(tab_id) = self.webview_tabs.remove(&webview_id) {
            let _ = self.residency.unregister_tab(tab_id);
        }
        self.webview_urls.remove(&webview_id);
        if self.active_webview_id == Some(webview_id) {
            self.active_webview_id = None;
        }
    }

    pub(crate) fn log_state(&self, context: &str) {
        let cache_stats = self.navigation_cache.stats();
        info!(
            "Soliloquy optimizations {context}: active_webview={:?} tracked_webviews={} pending_prefetches={} cache_entries={} cache_bytes={}",
            self.active_webview_id,
            self.webview_tabs.len(),
            self.prefetch.pending_count(),
            cache_stats.entries,
            cache_stats.size,
        );
    }

    fn upsert_webview_url(
        &mut self,
        webview_id: WebViewId,
        url: &Url,
        priority: PrefetchPriority,
        stage: &str,
    ) {
        let url = url.as_str().to_string();
        let tab_id = self
            .webview_tabs
            .get(&webview_id)
            .copied()
            .unwrap_or_else(|| self.residency.register_tab(url.clone()));

        self.webview_tabs.insert(webview_id, tab_id);
        self.webview_urls.insert(webview_id, url.clone());
        self.active_webview_id = Some(webview_id);
        let _ = self.residency.touch_tab(tab_id);
        self.queue_prefetches(url.as_str(), priority);
        self.cache_navigation(&format!("webview:{webview_id:?}:{stage}"), &url);
    }

    fn queue_prefetches(&mut self, url: &str, priority: PrefetchPriority) {
        self.prefetch
            .request_prefetch(url.to_string(), ResourceType::Connection, priority);
        if let Ok(parsed) = Url::parse(url) {
            if let Some(host) = parsed.host_str() {
                self.prefetch.request_prefetch(
                    host.to_string(),
                    ResourceType::Dns,
                    priority,
                );
            }
        }
    }

    fn cache_navigation(&mut self, key: &str, url: &str) {
        self.navigation_cache.insert(
            key.to_string(),
            CachedResource::new(url.to_string(), url.len(), 1),
        );
    }
}

impl Default for SoliloquyOptimizations {
    fn default() -> Self {
        Self::new()
    }
}
