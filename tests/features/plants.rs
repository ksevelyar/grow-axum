use super::support::{
    create_application, create_entry, create_log, create_plant, read_json_body,
    register_and_sign_in, send_json_request,
};
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;

#[sqlx::test]
async fn user_creates_a_plant(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let session_cookie = register_and_sign_in(&application, "grower@example.com").await;

    let response = send_json_request(
        &application,
        Method::POST,
        "/plants",
        json!({"name": "Tomato"}),
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let plant = read_json_body(response).await;
    assert_eq!(plant["name"], "Tomato");
    assert!(plant["id"].is_i64());
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_sees_a_plants_grow_logs_ordered_by_latest_entry(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let session_cookie = register_and_sign_in(&application, "grower@example.com").await;
    let plant = create_plant(&application, &session_cookie, "Mustard").await;
    let plant_id = plant["id"].as_i64().unwrap();
    let newest_log = create_log(&application, &session_cookie, plant_id, "Mustard 0").await;
    let older_log = create_log(&application, &session_cookie, plant_id, "Mustard 1").await;
    create_log(&application, &session_cookie, plant_id, "Mustard 4L").await;
    create_entry(
        &application,
        &session_cookie,
        newest_log["id"].as_i64().unwrap(),
        "2024-03-02T00:00:00Z",
        "vegetation",
    )
    .await;
    create_entry(
        &application,
        &session_cookie,
        older_log["id"].as_i64().unwrap(),
        "2024-03-01T00:00:00Z",
        "seedling",
    )
    .await;

    let response = send_json_request(
        &application,
        Method::GET,
        &format!("/plants/{plant_id}"),
        Value::Null,
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let plant_detail = read_json_body(response).await;
    let grow_log_names: Vec<&str> = plant_detail["grow_logs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|grow_log| grow_log["name"].as_str().unwrap())
        .collect();
    assert_eq!(grow_log_names, ["Mustard 0", "Mustard 1", "Mustard 4L"]);
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_cannot_delete_a_plant_with_grow_logs(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let session_cookie = register_and_sign_in(&application, "grower@example.com").await;
    let plant = create_plant(&application, &session_cookie, "Mustard").await;
    create_log(
        &application,
        &session_cookie,
        plant["id"].as_i64().unwrap(),
        "Spring grow",
    )
    .await;

    let response = send_json_request(
        &application,
        Method::DELETE,
        &format!("/plants/{}", plant["id"]),
        Value::Null,
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(read_json_body(response).await, json!({"error": "conflict"}));
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}
