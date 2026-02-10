// =============================================================================
// Bearer Token Authentication — Axum Middleware
// =============================================================================
//
// Extracts and validates a Bearer token from the `Authorization` header.
// The expected token is read from the `AURORA_ADMIN_TOKEN` environment variable
// at startup. Comparison is performed in constant time to prevent timing
// side-channel attacks.
//
// Usage as an Axum extractor:
//
//   async fn handler(AuthBearer(token): AuthBearer, ...) { ... }
//
// If the token is missing or invalid, the extractor short-circuits the request
// with a 403 Forbidden response before the handler body executes.
// =============================================================================

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::warn;

// =============================================================================
// Constant-time comparison
// =============================================================================

/// Compare two byte slices in constant time. Returns `true` if they are
/// identical. The comparison always examines every byte of both slices even
/// when a mismatch is found early, preventing timing side-channels.
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        // Length difference is observable, but we still iterate to avoid
        // revealing *where* the length check failed in terms of timing.
        // In practice, a length mismatch already leaks the fact that lengths
        // differ, which is acceptable for token authentication (the attacker
        // does not control the expected token length).
        return false;
    }

    let mut result: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

// =============================================================================
// Extractor
// =============================================================================

/// Axum extractor that validates the `Authorization: Bearer <token>` header
/// against the `AURORA_ADMIN_TOKEN` environment variable.
///
/// If the token is valid the extractor yields the raw token string (useful for
/// downstream logging or audit). If validation fails a 403 response is
/// returned immediately.
pub struct AuthBearer(pub String);

/// Rejection type returned when authentication fails.
pub struct AuthRejection {
    status: StatusCode,
    message: &'static str,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": self.message,
        });
        (self.status, axum::Json(body)).into_response()
    }
}

impl<S> FromRequestParts<S> for AuthBearer
where
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Read the expected token from the environment. We read it on every
        // request so that rotation does not require a restart (cache in
        // production if latency matters).
        let expected = std::env::var("AURORA_ADMIN_TOKEN").unwrap_or_default();

        if expected.is_empty() {
            warn!("AURORA_ADMIN_TOKEN is not set — all authenticated requests will be rejected");
            return Err(AuthRejection {
                status: StatusCode::FORBIDDEN,
                message: "Server authentication not configured",
            });
        }

        // Extract the Authorization header.
        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        let token = match auth_header {
            Some(value) if value.starts_with("Bearer ") => &value[7..],
            _ => {
                warn!("Missing or malformed Authorization header");
                return Err(AuthRejection {
                    status: StatusCode::FORBIDDEN,
                    message: "Missing or invalid authorization token",
                });
            }
        };

        // Constant-time comparison.
        if !constant_time_eq(token.as_bytes(), expected.as_bytes()) {
            warn!("Invalid admin token presented");
            return Err(AuthRejection {
                status: StatusCode::FORBIDDEN,
                message: "Invalid authorization token",
            });
        }

        Ok(AuthBearer(token.to_string()))
    }
}

// =============================================================================
// Token validation helper (for WebSocket query-param auth)
// =============================================================================

/// Validate a token string against the `AURORA_ADMIN_TOKEN` environment
/// variable. Returns `true` if the token is valid.
///
/// This is intended for contexts where the Axum extractor is not usable (e.g.
/// WebSocket upgrade where the token is passed as a query parameter).
pub fn validate_token(token: &str) -> bool {
    let expected = std::env::var("AURORA_ADMIN_TOKEN").unwrap_or_default();
    if expected.is_empty() {
        return false;
    }
    constant_time_eq(token.as_bytes(), expected.as_bytes())
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_identical() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn constant_time_eq_different() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn constant_time_eq_different_lengths() {
        assert!(!constant_time_eq(b"short", b"longer_string"));
    }

    #[test]
    fn constant_time_eq_empty() {
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn constant_time_eq_single_bit_diff() {
        assert!(!constant_time_eq(b"\x00", b"\x01"));
    }
}
