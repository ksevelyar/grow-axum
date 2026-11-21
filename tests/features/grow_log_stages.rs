use super::support::{
    create_application, create_entry, create_log, create_plant, read_json_body,
    register_and_sign_in, send_json_request,
};
use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;

async fn create_mustard_grow(application: &axum::Router) -> (String, i64) {
    let session_cookie = register_and_sign_in(application, "grower@example.com").await;
    let plant = create_plant(application, &session_cookie, "Mustard").await;
    let plant_id = plant["id"].as_i64().expect("plant has an identifier");
    let grow_log = create_log(application, &session_cookie, plant_id, "Mustard").await;
    let grow_log_id = grow_log["id"].as_i64().expect("grow log has an identifier");
    (session_cookie, grow_log_id)
}

#[sqlx::test]
async fn user_cannot_create_entry_on_occupied_date(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let (session_cookie, grow_log_id) = create_mustard_grow(&application).await;
    create_entry(
        &application,
        &session_cookie,
        grow_log_id,
        "2026-04-15T00:00:00Z",
        "vegetation",
    )
    .await;
    let response = send_json_request(
        &application,
        Method::POST,
        &format!("/logs/{grow_log_id}/entries"),
        json!({"occurred_at": "2026-04-15T12:00:00Z", "stage": "vegetation"}),
        Some(&session_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "validation failed", "fields": [{"field": "occurred_at", "message": "an entry already exists for this date"}]})
    );
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_cannot_move_entry_to_occupied_date(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let (session_cookie, grow_log_id) = create_mustard_grow(&application).await;
    create_entry(
        &application,
        &session_cookie,
        grow_log_id,
        "2026-04-01T00:00:00Z",
        "vegetation",
    )
    .await;
    let entry = create_entry(
        &application,
        &session_cookie,
        grow_log_id,
        "2026-04-15T00:00:00Z",
        "vegetation",
    )
    .await;
    let response = send_json_request(
        &application,
        Method::PATCH,
        &format!("/entries/{}", entry["id"]),
        json!({"occurred_at": "2026-04-01T12:00:00Z", "stage": "vegetation"}),
        Some(&session_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "validation failed", "fields": [{"field": "occurred_at", "message": "an entry already exists for this date"}]})
    );
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_cannot_record_earlier_stage(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let (session_cookie, grow_log_id) = create_mustard_grow(&application).await;
    create_entry(
        &application,
        &session_cookie,
        grow_log_id,
        "2026-04-01T00:00:00Z",
        "vegetation",
    )
    .await;
    create_entry(
        &application,
        &session_cookie,
        grow_log_id,
        "2026-05-01T00:00:00Z",
        "bloom",
    )
    .await;
    let response = send_json_request(
        &application,
        Method::POST,
        &format!("/logs/{grow_log_id}/entries"),
        json!({"occurred_at": "2026-05-06T10:30:00Z", "stage": "vegetation"}),
        Some(&session_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "validation failed", "fields": [{"field": "stage", "message": "must follow the order of the other entries"}]})
    );
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_cannot_change_earlier_entry_to_later_stage(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let (session_cookie, grow_log_id) = create_mustard_grow(&application).await;
    let entry = create_entry(
        &application,
        &session_cookie,
        grow_log_id,
        "2026-04-01T10:30:00Z",
        "vegetation",
    )
    .await;
    create_entry(
        &application,
        &session_cookie,
        grow_log_id,
        "2026-04-15T00:00:00Z",
        "vegetation",
    )
    .await;
    let response = send_json_request(
        &application,
        Method::PATCH,
        &format!("/entries/{}", entry["id"]),
        json!({"occurred_at": "2026-04-01T10:30:00Z", "stage": "bloom"}),
        Some(&session_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        read_json_body(response).await,
        json!({"error": "validation failed", "fields": [{"field": "stage", "message": "must follow the order of the other entries"}]})
    );
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_skips_stage(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let (session_cookie, grow_log_id) = create_mustard_grow(&application).await;
    create_entry(
        &application,
        &session_cookie,
        grow_log_id,
        "2026-03-20T00:00:00Z",
        "germination",
    )
    .await;
    let response = send_json_request(
        &application,
        Method::POST,
        &format!("/logs/{grow_log_id}/entries"),
        json!({"occurred_at": "2026-04-01T10:30:00Z", "stage": "vegetation"}),
        Some(&session_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let entry = read_json_body(response).await;
    assert_eq!(entry["grow_log_id"], grow_log_id);
    assert_eq!(entry["stage"], "vegetation");
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}
