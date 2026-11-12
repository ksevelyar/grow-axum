use super::support::{
    create_application, create_entry, create_log, create_plant, read_json_body,
    register_and_sign_in, send_json_request,
};
use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test]
async fn user_adds_an_entry_to_a_grow_log(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let session_cookie = register_and_sign_in(&application, "grower@example.com").await;
    let plant = create_plant(&application, &session_cookie, "Mustard").await;
    let grow_log = create_log(
        &application,
        &session_cookie,
        plant["id"].as_i64().unwrap(),
        "Spring grow",
    )
    .await;

    let response = send_json_request(
        &application,
        Method::POST,
        &format!("/logs/{}/entries", grow_log["id"]),
        json!({
            "occurred_at": "2024-03-08T10:30:00Z",
            "stage": "seedling",
            "description": "First true leaves"
        }),
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let entry = read_json_body(response).await;
    assert_eq!(entry["grow_log_id"], grow_log["id"]);
    assert_eq!(entry["stage"], "seedling");
    assert_eq!(entry["description"], "First true leaves");
    assert_eq!(entry["photos"], json!([]));
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_updates_an_entry(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let session_cookie = register_and_sign_in(&application, "grower@example.com").await;
    let plant = create_plant(&application, &session_cookie, "Mustard").await;
    let grow_log = create_log(
        &application,
        &session_cookie,
        plant["id"].as_i64().unwrap(),
        "Spring grow",
    )
    .await;
    let entry = create_entry(
        &application,
        &session_cookie,
        grow_log["id"].as_i64().unwrap(),
        "2024-03-08T10:30:00Z",
        "seedling",
    )
    .await;

    let response = send_json_request(
        &application,
        Method::PATCH,
        &format!("/entries/{}", entry["id"]),
        json!({
            "occurred_at": "2024-03-08T10:30:00Z",
            "stage": "vegetation",
            "description": "Transplanted"
        }),
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let updated_entry = read_json_body(response).await;
    assert_eq!(updated_entry["stage"], "vegetation");
    assert_eq!(updated_entry["description"], "Transplanted");
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}
