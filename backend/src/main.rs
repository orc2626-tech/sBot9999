// =============================================================================
// Aurora Spot Nexus — Main Entry Point
// =============================================================================
//
// The engine starts in Demo + Paused mode for safety. Users must explicitly
// switch to Live mode via the dashboard or API.
// =============================================================================

// ── Module declarations ──────────────────────────────────────────────────────
mod absorption_detector;
mod api;
mod app_state;
mod arena;
mod binance;
mod cusum_detector;
mod decision_envelope;
mod execution;
mod exit;
mod futures_intel;
mod htf_analysis;
mod indicators;
mod market_data;
mod position_engine;
mod reconcile;
mod regime;
mod risk;
mod runtime_config;
mod signals;
mod smart_filters;
mod strategy;
mod trade_insurance;
mod types;

use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::app_state::AppState;
use crate::execution::ExecutionEngine;
use crate::runtime_config::RuntimeConfig;
use crate::strategy::StrategyEngine;
use crate::types::AccountMode;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── 1. Environment & config ──────────────────────────────────────────
    let _ = dotenv::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║        Aurora Spot Nexus — Starting Up                  ║");
    info!("╚══════════════════════════════════════════════════════════╝");

    let mut config = RuntimeConfig::load("runtime_config.json").unwrap_or_else(|e| {
        warn!(error = %e, "Failed to load config, using defaults");
        RuntimeConfig::default()
    });

    // SAFETY: Force Demo + Paused on startup.
    config.trading_mode = types::TradingMode::Paused;
    config.account_mode = AccountMode::Demo;

    // Override symbols from env if available.
    if let Ok(syms) = std::env::var("AURORA_SYMBOLS") {
        config.symbols = syms
            .split(',')
            .map(|s| s.trim().to_uppercase())
            .filter(|s| !s.is_empty())
            .collect();
    }
    if config.symbols.is_empty() {
        config.symbols = vec![
            "BTCUSDT".into(),
            "ETHUSDT".into(),
            "BNBUSDT".into(),
            "XRPUSDT".into(),
            "SOLUSDT".into(),
        ];
    }

    info!(symbols = ?config.symbols, "Configured trading pairs");
    info!(
        trading_mode = %config.trading_mode,
        account_mode = %config.account_mode,
        "Engine starting in SAFE mode (Demo + Paused)"
    );

    // ── 2. Build shared state ────────────────────────────────────────────
    let state = Arc::new(AppState::new(config));

    // ── 3. Build Binance client ──────────────────────────────────────────
    let api_key = std::env::var("BINANCE_API_KEY").unwrap_or_default();
    let api_secret = std::env::var("BINANCE_API_SECRET").unwrap_or_default();
    let binance_client = Arc::new(binance::client::BinanceClient::new(api_key, api_secret));

    // ── 4. Spawn market data streams ─────────────────────────────────────
    let symbols = state.runtime_config.read().symbols.clone();

    for symbol in &symbols {
        // Kline 1m stream
        let cb = state.candle_buffer.clone();
        let sym = symbol.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) =
                    market_data::candle_buffer::run_kline_stream(&sym, "1m", &cb).await
                {
                    error!(symbol = %sym, error = %e, "Kline 1m stream error — reconnecting in 5s");
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        // Kline 5m stream
        let cb = state.candle_buffer.clone();
        let sym = symbol.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) =
                    market_data::candle_buffer::run_kline_stream(&sym, "5m", &cb).await
                {
                    error!(symbol = %sym, error = %e, "Kline 5m stream error — reconnecting in 5s");
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });

        // Trade stream
        {
            let procs = state.trade_processors.read();
            if let Some(tp) = procs.get(symbol) {
                let processor = tp.clone();
                let sym = symbol.clone();
                tokio::spawn(async move {
                    loop {
                        if let Err(e) =
                            market_data::trade_stream::run_trade_stream(&sym, &processor).await
                        {
                            error!(symbol = %sym, error = %e, "Trade stream error — reconnecting in 5s");
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                });
            }
        }

        // Orderbook stream
        let ob = state.orderbook_manager.clone();
        let sym = symbol.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = market_data::orderbook::run_depth_stream(&sym, &ob).await {
                    error!(symbol = %sym, error = %e, "Depth stream error — reconnecting in 5s");
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
    }

    info!(count = symbols.len(), "Market data streams launched");

    // ── 5. Start the API server ──────────────────────────────────────────
    let api_state = state.clone();
    let bind_addr =
        std::env::var("AURORA_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:3001".into());
    let bind_addr_clone = bind_addr.clone();

    tokio::spawn(async move {
        let app = api::rest::router(api_state);
        let listener = tokio::net::TcpListener::bind(&bind_addr_clone)
            .await
            .expect("Failed to bind API server");
        info!(addr = %bind_addr_clone, "API server listening");
        axum::serve(listener, app)
            .await
            .expect("API server failed");
    });

    // ── 6. Execution engine ──────────────────────────────────────────────
    let exec_engine = Arc::new(ExecutionEngine::new(
        binance_client.clone(),
        state.position_manager.clone(),
        state.risk_engine.clone(),
    ));

    // ── 7. Strategy loop (every 5 seconds) ───────────────────────────────
    let strat_state = state.clone();
    let strat_exec = exec_engine.clone();
    tokio::spawn(async move {
        // Wait for initial data
        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        info!("Strategy loop starting");

        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;

            let syms = strat_state.runtime_config.read().symbols.clone();
            let is_demo =
                strat_state.runtime_config.read().account_mode == AccountMode::Demo;

            for symbol in &syms {
                let (envelope, proposal) =
                    StrategyEngine::evaluate_symbol(&strat_state, symbol);
                strat_state.push_decision(envelope);

                if let Some(prop) = proposal {
                    let result = strat_exec
                        .execute_proposal(
                            &prop.symbol,
                            &prop.side,
                            prop.entry_price,
                            prop.quantity,
                            prop.stop_loss,
                            prop.take_profit_1,
                            prop.take_profit_2,
                            is_demo,
                        )
                        .await;
                    info!(symbol = %prop.symbol, side = %prop.side, result = %result, "trade execution result");
                }
            }
        }
    });

    // ── 8. Exit monitor loop ─────────────────────────────────────────────
    let exit_state = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;

            // Update prices
            let open_positions = exit_state.position_manager.get_open_positions();
            for pos in &open_positions {
                let procs = exit_state.trade_processors.read();
                if let Some(tp) = procs.get(&pos.symbol) {
                    let price = tp.last_price();
                    if price > 0.0 {
                        exit_state.position_manager.update_price(&pos.symbol, price);
                    }
                }
            }

            // Check exits
            let exits = exit_state.position_manager.check_exits();
            for (position_id, reason) in exits {
                let open = exit_state.position_manager.get_open_positions();
                if let Some(pos) = open.iter().find(|p| p.id == position_id) {
                    let close_price = pos.current_price;
                    if let Some(pnl) = exit_state.position_manager.close_position(
                        &position_id,
                        &reason,
                        close_price,
                    ) {
                        exit_state.risk_engine.record_trade_result(pnl);
                        exit_state.increment_version();
                        info!(position_id = %position_id, reason = %reason, pnl, "position exited");
                    }
                }
            }
        }
    });

    // ── 9. Reconciliation loop ───────────────────────────────────────────
    let recon_state = state.clone();
    let recon_client = binance_client.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;

            if recon_state.runtime_config.read().account_mode == AccountMode::Demo {
                continue;
            }

            match recon_client.get_account().await {
                Ok(account_info) => {
                    if let Some(balances) =
                        account_info.get("balances").and_then(|v| v.as_array())
                    {
                        let mut new_balances = Vec::new();
                        for b in balances {
                            let asset =
                                b.get("asset").and_then(|v| v.as_str()).unwrap_or("");
                            let free: f64 = b
                                .get("free")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0.0);
                            let locked: f64 = b
                                .get("locked")
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0.0);
                            if free > 0.0 || locked > 0.0 {
                                new_balances.push(types::BalanceInfo {
                                    asset: asset.to_string(),
                                    free,
                                    locked,
                                });
                            }
                        }
                        *recon_state.balances.write() = new_balances;
                        *recon_state.last_reconcile_ok.write() =
                            Some(std::time::Instant::now());
                        *recon_state.last_reconcile_error.write() = None;
                        recon_state.increment_version();
                    }
                }
                Err(e) => {
                    *recon_state.last_reconcile_error.write() = Some(format!("{e}"));
                    warn!(error = %e, "reconciliation failed");
                }
            }
        }
    });

    // ── 10. Regime detection loop ────────────────────────────────────────
    let regime_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let syms = regime_state.runtime_config.read().symbols.clone();
            if let Some(symbol) = syms.first() {
                let key = market_data::CandleKey {
                    symbol: symbol.clone(),
                    interval: "5m".to_string(),
                };
                let candles = regime_state.candle_buffer.get_closed_candles(&key, 100);
                if candles.len() >= 50 {
                    regime_state.regime_detector.write().update(&candles);
                    regime_state.increment_version();
                }
            }
        }
    });

    info!("All subsystems running. Press Ctrl+C to stop.");

    // ── 11. Graceful shutdown ────────────────────────────────────────────
    tokio::signal::ctrl_c().await?;
    warn!("Shutdown signal received — stopping gracefully");

    if let Err(e) = state.runtime_config.read().save("runtime_config.json") {
        error!(error = %e, "Failed to save runtime config on shutdown");
    }

    info!("Aurora Spot Nexus shut down complete.");
    Ok(())
}
