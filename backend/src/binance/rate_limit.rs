// =============================================================================
// Rate-Limit Tracker — monitors Binance API usage to avoid 429s
// =============================================================================
//
// Binance enforces multiple rate limits:
//   - Request weight: 1200 per minute (we hard-cap ourselves at 1000).
//   - Order rate:     10 per second and 200 000 per day.
//
// The tracker reads the `X-MBX-USED-WEIGHT-1M` response header after every
// request and keeps atomic counters that any thread may query lock-free.
// =============================================================================

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use tracing::{debug, warn};

/// Hard ceiling at which we refuse to send additional requests.
const WEIGHT_HARD_LIMIT: u32 = 1000;
/// Soft warning threshold.
const WEIGHT_WARN_THRESHOLD: u32 = 800;

/// Maximum orders per 10-second window.
const ORDER_10S_LIMIT: u32 = 10;
/// Maximum orders per day.
const ORDER_1D_LIMIT: u32 = 200_000;

/// Thread-safe rate-limit tracker backed by atomic counters.
pub struct RateLimitTracker {
    used_weight_1m: AtomicU32,
    order_count_10s: AtomicU32,
    order_count_1d: AtomicU32,
}

/// Immutable snapshot of the current rate-limit state (suitable for
/// serialisation into a dashboard payload).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitSnapshot {
    pub used_weight_1m: u32,
    pub order_count_10s: u32,
    pub order_count_1d: u32,
}

impl RateLimitTracker {
    /// Create a new tracker with all counters at zero.
    pub fn new() -> Self {
        Self {
            used_weight_1m: AtomicU32::new(0),
            order_count_10s: AtomicU32::new(0),
            order_count_1d: AtomicU32::new(0),
        }
    }

    // -------------------------------------------------------------------------
    // Header-based updates
    // -------------------------------------------------------------------------

    /// Update internal counters from the HTTP response headers returned by
    /// Binance.  The most important header is `X-MBX-USED-WEIGHT-1M`.
    pub fn update_from_headers(&self, headers: &reqwest::header::HeaderMap) {
        if let Some(val) = headers.get("X-MBX-USED-WEIGHT-1M") {
            if let Ok(s) = val.to_str() {
                if let Ok(w) = s.parse::<u32>() {
                    let prev = self.used_weight_1m.swap(w, Ordering::Relaxed);
                    if w >= WEIGHT_WARN_THRESHOLD && prev < WEIGHT_WARN_THRESHOLD {
                        warn!(
                            used_weight = w,
                            hard_limit = WEIGHT_HARD_LIMIT,
                            "rate-limit weight crossed warning threshold"
                        );
                    } else if w >= WEIGHT_WARN_THRESHOLD {
                        warn!(used_weight = w, "rate-limit weight remains above warning threshold");
                    }
                    debug!(used_weight_1m = w, "rate-limit weight updated from header");
                }
            }
        }

        // Binance also returns X-MBX-ORDER-COUNT-10S / -1D in some responses.
        if let Some(val) = headers.get("X-MBX-ORDER-COUNT-10S") {
            if let Ok(s) = val.to_str() {
                if let Ok(c) = s.parse::<u32>() {
                    self.order_count_10s.store(c, Ordering::Relaxed);
                }
            }
        }

        if let Some(val) = headers.get("X-MBX-ORDER-COUNT-1D") {
            if let Ok(s) = val.to_str() {
                if let Ok(c) = s.parse::<u32>() {
                    self.order_count_1d.store(c, Ordering::Relaxed);
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Pre-flight checks
    // -------------------------------------------------------------------------

    /// Return `true` if we can afford to spend `weight` more request weight
    /// without exceeding the hard limit.
    pub fn can_send_request(&self, weight: u32) -> bool {
        let current = self.used_weight_1m.load(Ordering::Relaxed);
        let allowed = current + weight <= WEIGHT_HARD_LIMIT;
        if !allowed {
            warn!(
                current_weight = current,
                requested_weight = weight,
                hard_limit = WEIGHT_HARD_LIMIT,
                "request blocked — would exceed rate-limit"
            );
        }
        allowed
    }

    /// Return `true` if we can place another order without violating the 10 s
    /// or daily order limit.
    pub fn can_place_order(&self) -> bool {
        let count_10s = self.order_count_10s.load(Ordering::Relaxed);
        let count_1d = self.order_count_1d.load(Ordering::Relaxed);

        if count_10s >= ORDER_10S_LIMIT {
            warn!(
                count_10s,
                limit = ORDER_10S_LIMIT,
                "order blocked — 10 s order limit reached"
            );
            return false;
        }
        if count_1d >= ORDER_1D_LIMIT {
            warn!(
                count_1d,
                limit = ORDER_1D_LIMIT,
                "order blocked — daily order limit reached"
            );
            return false;
        }
        true
    }

    /// Manually increment the order counters (useful when placing orders
    /// locally before the exchange responds with updated headers).
    pub fn record_order_sent(&self) {
        self.order_count_10s.fetch_add(1, Ordering::Relaxed);
        self.order_count_1d.fetch_add(1, Ordering::Relaxed);
    }

    /// Reset the 10-second order counter (call from a periodic timer).
    pub fn reset_10s_counter(&self) {
        self.order_count_10s.store(0, Ordering::Relaxed);
    }

    /// Reset the 1-minute weight counter (call from a periodic timer).
    pub fn reset_1m_weight(&self) {
        self.used_weight_1m.store(0, Ordering::Relaxed);
    }

    /// Reset the daily order counter (call at midnight UTC).
    pub fn reset_daily_counter(&self) {
        self.order_count_1d.store(0, Ordering::Relaxed);
    }

    // -------------------------------------------------------------------------
    // Snapshot
    // -------------------------------------------------------------------------

    /// Produce a serialisable snapshot of the current counters.
    pub fn snapshot(&self) -> RateLimitSnapshot {
        RateLimitSnapshot {
            used_weight_1m: self.used_weight_1m.load(Ordering::Relaxed),
            order_count_10s: self.order_count_10s.load(Ordering::Relaxed),
            order_count_1d: self.order_count_1d.load(Ordering::Relaxed),
        }
    }
}

impl Default for RateLimitTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for RateLimitTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateLimitTracker")
            .field("used_weight_1m", &self.used_weight_1m.load(Ordering::Relaxed))
            .field("order_count_10s", &self.order_count_10s.load(Ordering::Relaxed))
            .field("order_count_1d", &self.order_count_1d.load(Ordering::Relaxed))
            .finish()
    }
}
