use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy)]
pub(super) enum CacheLookupRoute {
    RequestId,
    ProjectPath,
    FallbackRoute,
}

#[derive(Debug, Default)]
struct CacheRouteMetrics {
    lookups: AtomicU64,
    hits: AtomicU64,
}

impl CacheRouteMetrics {
    fn record_lookup(&self, hit: bool) {
        self.lookups.fetch_add(1, Ordering::Relaxed);
        if hit {
            self.hits.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn snapshot(&self) -> serde_json::Value {
        let lookups = self.lookups.load(Ordering::Relaxed);
        let hits = self.hits.load(Ordering::Relaxed);

        serde_json::json!({
            "lookups": lookups,
            "hits": hits,
            "misses": lookups.saturating_sub(hits),
            "hit_rate_percent": hit_rate_percent(hits, lookups),
        })
    }
}

#[derive(Debug, Default)]
pub(super) struct CacheMetrics {
    lookups: AtomicU64,
    hits: AtomicU64,
    writes: AtomicU64,
    pruned: AtomicU64,
    active_registry_fallback_hits: AtomicU64,
    request_id: CacheRouteMetrics,
    project_path: CacheRouteMetrics,
    fallback_route: CacheRouteMetrics,
}

impl CacheMetrics {
    pub(super) fn record_lookup(&self, route: CacheLookupRoute, hit: bool) {
        self.lookups.fetch_add(1, Ordering::Relaxed);
        if hit {
            self.hits.fetch_add(1, Ordering::Relaxed);
        }

        match route {
            CacheLookupRoute::RequestId => self.request_id.record_lookup(hit),
            CacheLookupRoute::ProjectPath => self.project_path.record_lookup(hit),
            CacheLookupRoute::FallbackRoute => self.fallback_route.record_lookup(hit),
        }
    }

    pub(super) fn record_write_count(&self, count: usize) {
        if count > 0 {
            self.writes.fetch_add(count as u64, Ordering::Relaxed);
        }
    }

    pub(super) fn record_pruned_count(&self, count: usize) {
        if count > 0 {
            self.pruned.fetch_add(count as u64, Ordering::Relaxed);
        }
    }

    pub(super) fn record_active_registry_fallback_hit(&self) {
        self.active_registry_fallback_hits
            .fetch_add(1, Ordering::Relaxed);
    }

    pub(super) fn snapshot(&self) -> serde_json::Value {
        let lookups = self.lookups.load(Ordering::Relaxed);
        let hits = self.hits.load(Ordering::Relaxed);

        serde_json::json!({
            "lookups": lookups,
            "hits": hits,
            "misses": lookups.saturating_sub(hits),
            "hit_rate_percent": hit_rate_percent(hits, lookups),
            "writes": self.writes.load(Ordering::Relaxed),
            "pruned": self.pruned.load(Ordering::Relaxed),
            "active_registry_fallback_hits": self.active_registry_fallback_hits.load(Ordering::Relaxed),
            "routes": {
                "request_id": self.request_id.snapshot(),
                "project_path": self.project_path.snapshot(),
                "fallback_route": self.fallback_route.snapshot(),
            },
        })
    }
}

pub(super) static MCP_STATE_CACHE_METRICS: once_cell::sync::Lazy<CacheMetrics> =
    once_cell::sync::Lazy::new(CacheMetrics::default);
pub(super) static MCP_ACTION_CACHE_METRICS: once_cell::sync::Lazy<CacheMetrics> =
    once_cell::sync::Lazy::new(CacheMetrics::default);

fn epoch_utc() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::<chrono::Utc>::from(std::time::UNIX_EPOCH)
}

fn hit_rate_percent(hits: u64, lookups: u64) -> f64 {
    if lookups == 0 {
        return 0.0;
    }

    ((hits as f64 * 10_000.0) / lookups as f64).round() / 100.0
}

fn cache_metrics_for(cache_name: &str) -> Option<&'static CacheMetrics> {
    match cache_name {
        "mcp_state" => Some(&MCP_STATE_CACHE_METRICS),
        "mcp_action" => Some(&MCP_ACTION_CACHE_METRICS),
        _ => None,
    }
}

pub(super) fn record_cache_write_count(cache_name: &str, count: usize) {
    if let Some(metrics) = cache_metrics_for(cache_name) {
        metrics.record_write_count(count);
    }
}

pub(super) fn mark_json_cache_keys(
    touched_at: &mut HashMap<String, chrono::DateTime<chrono::Utc>>,
    keys: &[String],
) {
    let now = chrono::Utc::now();
    for key in keys {
        touched_at.insert(key.clone(), now);
    }
}

pub(super) fn prune_json_cache(
    cache_name: &str,
    cache: &mut HashMap<String, serde_json::Value>,
    touched_at: &mut HashMap<String, chrono::DateTime<chrono::Utc>>,
    ttl_secs: i64,
    max_entries: usize,
) -> usize {
    let before = cache.len();
    let cutoff = chrono::Utc::now() - chrono::Duration::seconds(ttl_secs);
    let expired_or_orphaned_keys = touched_at
        .iter()
        .filter_map(|(key, touched)| {
            if *touched < cutoff || !cache.contains_key(key) {
                Some(key.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for key in expired_or_orphaned_keys {
        cache.remove(&key);
        touched_at.remove(&key);
    }

    if cache.len() > max_entries {
        let mut ordered = cache
            .keys()
            .map(|key| {
                (
                    key.clone(),
                    touched_at.get(key).cloned().unwrap_or_else(epoch_utc),
                )
            })
            .collect::<Vec<_>>();
        ordered.sort_by(|a, b| a.1.cmp(&b.1));

        for (key, _) in ordered.into_iter().take(cache.len() - max_entries) {
            cache.remove(&key);
            touched_at.remove(&key);
        }
    }

    let removed = before.saturating_sub(cache.len());
    if let Some(metrics) = cache_metrics_for(cache_name) {
        metrics.record_pruned_count(removed);
    }
    if removed > 0 {
        log::info!(
            "[Bridge] cache-prune: cache={}, removed={}, remaining={}, ttl_secs={}, max_entries={}",
            cache_name,
            removed,
            cache.len(),
            ttl_secs,
            max_entries
        );
    }
    removed
}
