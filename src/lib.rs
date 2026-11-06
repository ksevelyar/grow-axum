pub mod authentication;
pub mod error;
pub mod handlers;
pub mod users;

use axum::Router;
use axum::routing::{get, post};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Debug)]
pub struct AppState {
    pub pool: PgPool,
}

pub fn build_app(pool: PgPool) -> Router {
    let state = Arc::new(AppState { pool });

    Router::new()
        .route("/sessions", post(handlers::users::create_session))
        .route("/sessions/current", get(handlers::users::show_current_user))
        .route("/users", post(handlers::users::create))
        .with_state(state)
}
