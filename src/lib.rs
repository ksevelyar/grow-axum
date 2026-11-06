pub mod authentication;
pub mod error;
pub mod grow_logs;
pub mod handlers;
pub mod users;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderValue, Method, header};
use axum::routing::{get, patch, post};
use sqlx::PgPool;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[derive(Debug)]
pub struct AppState {
    pub pool: PgPool,
    pub photo_storage_directory: PathBuf,
}

pub fn build_app(pool: PgPool) -> Router {
    let photo_storage_directory = std::env::var_os("PHOTO_STORAGE_DIRECTORY")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("grow-axum-photos"));
    build_app_with_photo_storage_directory(pool, photo_storage_directory)
}

pub fn build_app_with_photo_storage_directory(
    pool: PgPool,
    photo_storage_directory: PathBuf,
) -> Router {
    let state = Arc::new(AppState {
        pool,
        photo_storage_directory,
    });
    let photo_upload_maximum_request_size = 10 * 1024 * 1024;
    let allowed_origins = std::env::var("CORS_ORIGINS")
        .unwrap_or_else(|_| "http://grow.localhost:3001,http://localhost:3001".to_owned())
        .split(',')
        .map(str::trim)
        .filter(|origin| !origin.is_empty())
        .map(|origin| {
            origin
                .parse::<HeaderValue>()
                .expect("CORS_ORIGINS must contain valid origins")
        })
        .collect::<Vec<_>>();
    let cors_layer = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE])
        .allow_credentials(true);

    Router::new()
        .route("/sessions", post(handlers::users::create_session))
        .route("/sessions/current", get(handlers::users::show_current_user))
        .route("/users", post(handlers::users::create))
        .route(
            "/plants",
            get(handlers::plants::index).post(handlers::plants::create),
        )
        .route(
            "/plants/{plant_id}",
            get(handlers::plants::show)
                .patch(handlers::plants::update)
                .delete(handlers::plants::delete),
        )
        .route(
            "/logs",
            get(handlers::grow_logs::index).post(handlers::grow_logs::create),
        )
        .route(
            "/logs/{log_id}",
            get(handlers::grow_logs::show)
                .patch(handlers::grow_logs::update)
                .delete(handlers::grow_logs::delete),
        )
        .route("/logs/{log_id}/entries", post(handlers::entries::create))
        .route(
            "/entries/{entry_id}",
            patch(handlers::entries::update).delete(handlers::entries::delete),
        )
        .route(
            "/entries/{entry_id}/photos",
            post(handlers::photos::create)
                .layer(DefaultBodyLimit::max(photo_upload_maximum_request_size)),
        )
        .route(
            "/photos/{photo_id}",
            get(handlers::photos::show).delete(handlers::photos::delete),
        )
        .with_state(state)
        .layer(cors_layer)
}
