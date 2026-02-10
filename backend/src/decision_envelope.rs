// =============================================================================
// Decision Envelope — Auditable record of every trade/no-trade decision
// =============================================================================
//
// Every trade signal flows through a multi-layer pipeline.  The Decision
// Envelope captures the verdict from each layer so that every decision can
// be audited after the fact.
//
// The struct fields use `String` for verdicts (not Option<String>) to match
// the construction pattern in main.rs, where all fields are populated at
// creation time.
// =============================================================================

use serde::Serialize;

/// Complete auditable record of a trade decision, including all layer verdicts.
#[derive(Debug, Clone, Serialize)]
pub struct DecisionEnvelope {
    /// Unique identifier for this decision (UUID v4).
    pub id: String,

    /// Symbol the decision pertains to.
    pub symbol: String,

    /// "BUY" or "SELL".
    pub side: String,

    /// Name of the strategy that generated the signal.
    pub strategy_name: String,

    /// Data quality gate verdict ("PASS" / "FAIL").
    pub data_quality_verdict: String,

    /// Insurance layer verdict ("PASS" / "FAIL").
    pub insurance_verdict: String,

    /// Risk engine verdict ("PASS" / "FAIL").
    pub risk_verdict: String,

    /// Execution quality verdict ("PASS" / "FAIL").
    pub execution_quality_verdict: String,

    /// Final decision: "ALLOW", "BLOCK", "SKIP".
    pub final_decision: String,

    /// Which layer blocked the trade (if blocked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking_layer: Option<String>,

    /// Human-readable reason for the decision.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// ISO 8601 timestamp of when this decision was created.
    pub created_at: String,

    /// Smart filter verdicts snapshot (serialised JSON).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smart_filters: Option<serde_json::Value>,
}

impl DecisionEnvelope {
    /// Create a new decision envelope that allows the trade.
    pub fn allow(
        symbol: impl Into<String>,
        side: impl Into<String>,
        strategy_name: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: symbol.into(),
            side: side.into(),
            strategy_name: strategy_name.into(),
            data_quality_verdict: "PASS".to_string(),
            insurance_verdict: "PASS".to_string(),
            risk_verdict: "PASS".to_string(),
            execution_quality_verdict: "PASS".to_string(),
            final_decision: "ALLOW".to_string(),
            blocking_layer: None,
            reason: None,
            created_at: chrono::Utc::now().to_rfc3339(),
            smart_filters: None,
        }
    }

    /// Create a blocked decision.
    pub fn blocked(
        symbol: impl Into<String>,
        side: impl Into<String>,
        strategy_name: impl Into<String>,
        blocking_layer: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let layer = blocking_layer.into();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            symbol: symbol.into(),
            side: side.into(),
            strategy_name: strategy_name.into(),
            data_quality_verdict: "PASS".to_string(),
            insurance_verdict: "PASS".to_string(),
            risk_verdict: "PASS".to_string(),
            execution_quality_verdict: "PASS".to_string(),
            final_decision: "BLOCK".to_string(),
            blocking_layer: Some(layer),
            reason: Some(reason.into()),
            created_at: chrono::Utc::now().to_rfc3339(),
            smart_filters: None,
        }
    }
}
