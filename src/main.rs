use axum::{
    error_handling::HandleErrorLayer, extract::Path, http::StatusCode, response::IntoResponse,
    routing::post, Json, Router, Extension,
};
use axum_macros::debug_handler;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{sync::Notify, time::timeout};
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;

type State = Arc<Mutex<HashMap<String, Arc<Notify>>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let state = State::default();

    // Compose the routes
    let app = Router::new()
        .route("/wait-for-second-party/:id", post(sync_point))
        // Add middleware to all routes
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(|error: BoxError| async move {
                    if error.is::<tower::timeout::error::Elapsed>() {
                        Ok(StatusCode::REQUEST_TIMEOUT)
                    } else {
                        Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Unhandled internal error: {}", error),
                        ))
                    }
                }))
                .timeout(Duration::from_secs(15))
                .layer(TraceLayer::new_for_http())
                .into_inner(),
        )
        .layer(Extension(state));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Response {
    status: String,
    msg: String,
}
#[debug_handler]
async fn sync_point(Path(id): Path<String>, Extension(state): Extension<State>) -> impl IntoResponse {
    // Check if the second party is already waiting
    let notify = {
        let mut state = state.lock().unwrap();
        if let Some(notify) = state.remove(&id) {
            notify.notify_one();
            
            return Json(Response {
                status: "ok".to_string(),
                msg: "Second party arrived".into(),
            });
        } else {
            let notify = Arc::new(Notify::new());
            state.insert(id, notify.clone());
            notify
        }
    };

    if timeout(Duration::from_secs(10), notify.notified()).await.is_ok(){
        return Json(Response {
            status: "ok".to_string(),
            msg: "Second party arrived".into(),
        });
    } else {
        return Json(Response {
            status: "error".to_string(),
            msg: "Timeout".into(),
        });
    }
}
