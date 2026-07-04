//! JSON error responses for `/api/v2/*` handlers.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    BadRequest(String),
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl ApiError {
    fn status_and_code(&self) -> (StatusCode, &'static str) {
        match self {
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Self::Unauthorized(_) => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Self::Forbidden(_) => (StatusCode::FORBIDDEN, "forbidden"),
            Self::BadRequest(_) => (StatusCode::BAD_REQUEST, "bad_request"),
            Self::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }

    fn message(&self) -> String {
        match self {
            Self::NotFound(m)
            | Self::Unauthorized(m)
            | Self::Forbidden(m)
            | Self::BadRequest(m)
            | Self::Internal(m) => m.clone(),
        }
    }

    /// Map shared `FnError` messages to HTTP API errors.
    pub fn from_server_fn(err: crate::server::fn_error::FnError) -> Self {
        let msg = err.to_string();
        let lower = msg.to_ascii_lowercase();
        if lower == "not found" {
            Self::Forbidden(msg)
        } else if lower.contains("not found") {
            Self::NotFound(msg)
        } else if lower.contains("sign in required") {
            Self::Unauthorized(msg)
        } else {
            Self::BadRequest(msg)
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code) = self.status_and_code();
        let body = Json(ErrorBody {
            error: ErrorDetail {
                code,
                message: self.message(),
            },
        });
        (status, body).into_response()
    }
}
