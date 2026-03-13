use axum::{
    Router,
    routing::{get, post},
};
use crate::state::AppState;
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(crate::handlers::tenants::list_tenants))
        .route("/switch", post(crate::handlers::tenants::switch_tenant))
}
