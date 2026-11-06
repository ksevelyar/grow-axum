use grow_axum::build_app_with_photo_storage_directory;
use sqlx::postgres::PgPoolOptions;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let photo_storage_directory = std::env::var_os("STORAGE_DIRECTORY")
        .map(PathBuf::from)
        .expect("STORAGE_DIRECTORY must be set");
    tokio::fs::create_dir_all(&photo_storage_directory)
        .await
        .expect("failed to create photo storage directory");
    let pool = PgPoolOptions::new()
        .connect(&database_url)
        .await
        .expect("failed to connect to database");
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .expect("failed to bind listener");
    axum::serve(
        listener,
        build_app_with_photo_storage_directory(pool, photo_storage_directory),
    )
    .await
    .expect("server failed");
}
