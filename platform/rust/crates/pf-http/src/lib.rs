use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use pf_errors::ProblemDetails;
use serde_json::json;

pub fn health_router() -> Router {
    Router::new()
        .route("/healthz", get(health_handler))
        .route("/readyz", get(ready_handler))
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "timestamp": chrono_now() }))
}

async fn ready_handler() -> Json<serde_json::Value> {
    Json(json!({ "status": "ready", "timestamp": chrono_now() }))
}

fn chrono_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub struct AppError(pub pf_errors::PlatformError);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let problem: ProblemDetails = self.0.into();
        let status =
            StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (status, Json(problem)).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<pf_errors::PlatformError>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_routes() {
        let router = health_router();
        let _service = router.into_make_service();
    }
}
