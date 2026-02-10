// =============================================================================
// REST API Endpoints — Axum 0.7
// =============================================================================
//
// All endpoints live under `/api/v1/`. Public endpoints (health) require no
// authentication. All other endpoints require a valid Bearer token checked via
// the `AuthBearer` extractor.
//
// CORS is configured permissively for development; tighten `allowed_origins`
// in production.
// =============================================================================

use std::sync::Arc;

use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use crate::api::auth::AuthBearer;
use crate::app_state::AppState;
use crate::types::{AccountMode, TradingMode};

// =============================================================================
// Router construction
// =============================================================================

/// Build the full REST API router with CORS middleware and shared state.
pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // ── Public ──────────────────────────────────────────────────
        .route("/api/v1/health", get(health))
        // ── Authenticated ───────────────────────────────────────────
        .route("/api/v1/state", get(full_state))
        .route("/api/v1/positions", get(positions))
        .route("/api/v1/regime", get(regime))
        .route("/api/v1/decisions", get(decisions))
        .route("/api/v1/feature-flags", get(get_feature_flags))
        .route("/api/v1/feature-flags", post(set_feature_flags))
        .route("/api/v1/control/pause", post(control_pause))
        .route("/api/v1/control/resume", post(control_resume))
        .route("/api/v1/control/kill", post(control_kill))
        .route("/api/v1/control/account-mode", post(control_account_mode))
        .route("/api/v1/heartbeat", post(heartbeat))
        .route("/api/v1/trade-journal", get(trade_journal))
        .route("/api/v1/trade-journal/stats", get(trade_journal_stats))
        // ── WebSocket (handled separately in ws module but mounted here) ─
        .route("/api/v1/ws", get(crate::api::ws::ws_handler))
        // ── Middleware & State ───────────────────────────────────────
        .layer(cors)
        .with_state(state)
}

// =============================================================================
// Health (public)
// =============================================================================

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    state_version: u64,
    server_time: i64,
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let resp = HealthResponse {
        status: "ok",
        state_version: state.current_state_version(),
        server_time: chrono::Utc::now().timestamp_millis(),
    };
    Json(resp)
}

// =============================================================================
// Full state snapshot (authenticated)
// =============================================================================

async fn full_state(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let snapshot = state.build_snapshot();
    Json(snapshot)
}

// =============================================================================
// Positions (authenticated)
// =============================================================================

async fn positions(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let positions = state.position_manager.get_open_positions();
    Json(positions)
}

// =============================================================================
// Regime (authenticated)
// =============================================================================

async fn regime(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let regime_state = state.regime_detector.read().current_regime();
    match regime_state {
        Some(rs) => {
            let snapshot = serde_json::json!({
                "regime": rs.regime.to_string(),
                "adx": rs.adx,
                "bbw": rs.bbw,
                "hurst": rs.hurst,
                "entropy": rs.entropy,
                "confidence": rs.confidence,
                "regime_age_seconds": rs.regime_age_secs,
            });
            Json(snapshot).into_response()
        }
        None => {
            let body = serde_json::json!({ "regime": null, "message": "No regime data available yet" });
            Json(body).into_response()
        }
    }
}

// =============================================================================
// Decisions (authenticated)
// =============================================================================

async fn decisions(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let decisions = state.recent_decisions.read().clone();
    Json(decisions)
}

// =============================================================================
// Feature Flags (authenticated)
// =============================================================================

async fn get_feature_flags(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let config = state.runtime_config.read();
    let flags = serde_json::json!({
        "htf_gate": config.enable_htf_gate,
        "score_momentum": config.enable_score_momentum,
        "ofip": config.enable_ofip,
        "adaptive_threshold": config.enable_adaptive_threshold,
        "entropy_graduated": config.enable_entropy_graduated,
        "cusum": config.enable_cusum,
        "absorption": config.enable_absorption,
        "entropy_valley": config.enable_entropy_valley,
        "micro_trail": config.enable_micro_trail,
    });
    Json(flags)
}

#[derive(Deserialize)]
struct FeatureFlagUpdate {
    #[serde(default)]
    htf_gate: Option<bool>,
    #[serde(default)]
    score_momentum: Option<bool>,
    #[serde(default)]
    ofip: Option<bool>,
    #[serde(default)]
    adaptive_threshold: Option<bool>,
    #[serde(default)]
    entropy_graduated: Option<bool>,
    #[serde(default)]
    cusum: Option<bool>,
    #[serde(default)]
    absorption: Option<bool>,
    #[serde(default)]
    entropy_valley: Option<bool>,
    #[serde(default)]
    micro_trail: Option<bool>,
}

async fn set_feature_flags(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
    Json(update): Json<FeatureFlagUpdate>,
) -> impl IntoResponse {
    let mut config = state.runtime_config.write();
    let mut changes = Vec::new();

    macro_rules! apply_flag {
        ($update_field:ident, $config_field:ident) => {
            if let Some(val) = update.$update_field {
                if config.$config_field != val {
                    changes.push(format!(
                        "{}: {} -> {}",
                        stringify!($update_field),
                        config.$config_field,
                        val
                    ));
                    config.$config_field = val;
                }
            }
        };
    }

    apply_flag!(htf_gate, enable_htf_gate);
    apply_flag!(score_momentum, enable_score_momentum);
    apply_flag!(ofip, enable_ofip);
    apply_flag!(adaptive_threshold, enable_adaptive_threshold);
    apply_flag!(entropy_graduated, enable_entropy_graduated);
    apply_flag!(cusum, enable_cusum);
    apply_flag!(absorption, enable_absorption);
    apply_flag!(entropy_valley, enable_entropy_valley);
    apply_flag!(micro_trail, enable_micro_trail);

    if !changes.is_empty() {
        info!(changes = ?changes, "Feature flags updated");

        // Clone config and drop write lock before saving.
        let config_clone = config.clone();
        drop(config);

        // Save to disk (best-effort).
        if let Err(e) = config_clone.save("runtime_config.json") {
            warn!(error = %e, "Failed to save feature flags to disk");
        }

        state.increment_version();

        let mut response = serde_json::json!({
            "htf_gate": config_clone.enable_htf_gate,
            "score_momentum": config_clone.enable_score_momentum,
            "ofip": config_clone.enable_ofip,
            "adaptive_threshold": config_clone.enable_adaptive_threshold,
            "entropy_graduated": config_clone.enable_entropy_graduated,
            "cusum": config_clone.enable_cusum,
            "absorption": config_clone.enable_absorption,
            "entropy_valley": config_clone.enable_entropy_valley,
            "micro_trail": config_clone.enable_micro_trail,
        });
        if let Some(obj) = response.as_object_mut() {
            obj.insert(
                "changes".to_string(),
                serde_json::to_value(&changes).unwrap_or_default(),
            );
        }
        Json(response).into_response()
    } else {
        let flags_snapshot = serde_json::json!({
            "htf_gate": config.enable_htf_gate,
            "score_momentum": config.enable_score_momentum,
            "ofip": config.enable_ofip,
            "adaptive_threshold": config.enable_adaptive_threshold,
            "entropy_graduated": config.enable_entropy_graduated,
            "cusum": config.enable_cusum,
            "absorption": config.enable_absorption,
            "entropy_valley": config.enable_entropy_valley,
            "micro_trail": config.enable_micro_trail,
        });
        drop(config);

        let mut response = flags_snapshot;
        if let Some(obj) = response.as_object_mut() {
            obj.insert(
                "changes".to_string(),
                serde_json::Value::Array(vec![]),
            );
        }
        Json(response).into_response()
    }
}

// =============================================================================
// Control endpoints (authenticated)
// =============================================================================

#[derive(Serialize)]
struct ControlResponse {
    trading_mode: String,
    message: String,
}

async fn control_pause(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    {
        let mut config = state.runtime_config.write();
        config.trading_mode = TradingMode::Paused;
    }
    state.increment_version();
    info!("Trading PAUSED via API");

    Json(ControlResponse {
        trading_mode: "Paused".to_string(),
        message: "Trading paused".to_string(),
    })
}

async fn control_resume(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    {
        let mut config = state.runtime_config.write();
        config.trading_mode = TradingMode::Live;
    }
    state.increment_version();
    info!("Trading RESUMED via API");

    Json(ControlResponse {
        trading_mode: "Live".to_string(),
        message: "Trading resumed".to_string(),
    })
}

async fn control_kill(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    {
        let mut config = state.runtime_config.write();
        config.trading_mode = TradingMode::Killed;
    }
    state.increment_version();
    warn!("Trading KILLED via API");

    Json(ControlResponse {
        trading_mode: "Killed".to_string(),
        message: "Trading killed — manual restart required".to_string(),
    })
}

#[derive(Deserialize)]
struct AccountModeRequest {
    account_mode: String,
    #[serde(default)]
    confirm_live: bool,
}

#[derive(Serialize)]
struct AccountModeResponse {
    account_mode: String,
}

async fn control_account_mode(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
    Json(req): Json<AccountModeRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let mode = match req.account_mode.to_lowercase().as_str() {
        "demo" => AccountMode::Demo,
        "live" => {
            if !req.confirm_live {
                return Err((
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({
                        "error": "Switching to Live mode requires confirm_live: true",
                    })),
                ));
            }
            warn!("Switching to LIVE account mode via API");
            AccountMode::Live
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": format!("Invalid account mode: '{}'. Use 'demo' or 'live'.", req.account_mode),
                })),
            ));
        }
    };

    {
        let mut config = state.runtime_config.write();
        config.account_mode = mode;
    }
    state.increment_version();
    info!(account_mode = %mode, "Account mode changed via API");

    Ok(Json(AccountModeResponse {
        account_mode: mode.to_string(),
    }))
}

// =============================================================================
// Heartbeat (authenticated)
// =============================================================================

async fn heartbeat(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    *state.last_ws_user_event.write() = std::time::Instant::now();
    state.increment_version();

    Json(serde_json::json!({
        "status": "ok",
        "server_time": chrono::Utc::now().timestamp_millis(),
    }))
}

// =============================================================================
// Trade Journal (authenticated)
// =============================================================================

async fn trade_journal(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let closed = state.position_manager.get_closed_positions(500);
    Json(closed)
}

async fn trade_journal_stats(
    _auth: AuthBearer,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    let closed = state.position_manager.get_closed_positions(500);
    let total_trades = closed.len();
    if total_trades == 0 {
        return Json(serde_json::json!({
            "total_trades": 0,
            "win_rate": 0.0,
            "total_net_pnl": 0.0,
            "profit_factor": 0.0,
        }));
    }
    let wins = closed.iter().filter(|p| p.realized_pnl > 0.0).count();
    let win_rate = wins as f64 / total_trades as f64;
    let total_net_pnl: f64 = closed.iter().map(|p| p.realized_pnl).sum();
    let gross_profit: f64 = closed
        .iter()
        .filter(|p| p.realized_pnl > 0.0)
        .map(|p| p.realized_pnl)
        .sum();
    let gross_loss: f64 = closed
        .iter()
        .filter(|p| p.realized_pnl < 0.0)
        .map(|p| p.realized_pnl.abs())
        .sum();
    let profit_factor = if gross_loss > 0.0 {
        gross_profit / gross_loss
    } else if gross_profit > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };
    Json(serde_json::json!({
        "total_trades": total_trades,
        "win_rate": win_rate,
        "total_net_pnl": total_net_pnl,
        "profit_factor": profit_factor,
    }))
}
