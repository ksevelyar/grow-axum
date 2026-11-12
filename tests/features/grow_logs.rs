use super::support::{
    PHOTO_FIXTURE_BYTES, create_application, create_entry, create_log, create_photo, create_plant,
    read_json_body, register_and_sign_in, send_json_request,
};
use axum::http::{Method, StatusCode, header};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;

#[sqlx::test]
async fn user_creates_a_grow_log(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let session_cookie = register_and_sign_in(&application, "grower@example.com").await;
    let plant = create_plant(&application, &session_cookie, "Mustard").await;

    let response = send_json_request(
        &application,
        Method::POST,
        "/logs",
        json!({
            "plant_id": plant["id"],
            "name": "Spring grow",
            "started_at": "2024-03-01T09:00:00Z",
            "description": "Started indoors",
            "ended_at": null,
            "outcome": null
        }),
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let grow_log = read_json_body(response).await;
    assert_eq!(grow_log["plant_id"], plant["id"]);
    assert_eq!(grow_log["name"], "Spring grow");
    assert_eq!(grow_log["entries"], json!([]));
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_sees_grow_logs_ordered_by_latest_entry(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let session_cookie = register_and_sign_in(&application, "grower@example.com").await;
    let plant = create_plant(&application, &session_cookie, "Mustard").await;
    let plant_id = plant["id"].as_i64().unwrap();
    let mustard_log = create_log(&application, &session_cookie, plant_id, "Mustard").await;
    let clover_log = create_log(&application, &session_cookie, plant_id, "Clover").await;
    create_log(&application, &session_cookie, plant_id, "Sunflower").await;
    create_entry(
        &application,
        &session_cookie,
        mustard_log["id"].as_i64().unwrap(),
        "2024-03-03T00:00:00Z",
        "vegetation",
    )
    .await;
    create_entry(
        &application,
        &session_cookie,
        clover_log["id"].as_i64().unwrap(),
        "2024-03-02T00:00:00Z",
        "seedling",
    )
    .await;

    let response = send_json_request(
        &application,
        Method::GET,
        "/logs",
        Value::Null,
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let grow_logs = read_json_body(response).await;
    let grow_log_names: Vec<&str> = grow_logs
        .as_array()
        .unwrap()
        .iter()
        .map(|grow_log| grow_log["name"].as_str().unwrap())
        .collect();
    assert_eq!(grow_log_names, ["Mustard", "Clover", "Sunflower"]);
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_completes_a_grow_log(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let session_cookie = register_and_sign_in(&application, "grower@example.com").await;
    let plant = create_plant(&application, &session_cookie, "Mustard").await;
    let plant_id = plant["id"].as_i64().unwrap();
    let grow_log = create_log(&application, &session_cookie, plant_id, "Spring grow").await;
    let grow_log_id = grow_log["id"].as_i64().unwrap();

    let response = send_json_request(
        &application,
        Method::PATCH,
        &format!("/logs/{grow_log_id}"),
        json!({
            "plant_id": plant_id,
            "name": "Spring grow",
            "started_at": "2024-01-01T00:00:00Z",
            "ended_at": "2024-07-01T12:00:00Z",
            "outcome": "Harvested"
        }),
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let completed_grow_log = read_json_body(response).await;
    assert_eq!(completed_grow_log["ended_at"], "2024-07-01T12:00:00Z");
    assert_eq!(completed_grow_log["outcome"], "Harvested");
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_cannot_use_an_invalid_stage(pool: PgPool) {
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
        json!({"occurred_at": "2024-03-08T10:30:00Z", "stage": "flowering"}),
        Some(&session_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        read_json_body(response).await["fields"][0]["field"],
        "stage"
    );
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_uploads_a_photo(pool: PgPool) {
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

    let response = create_photo(
        &application,
        &session_cookie,
        entry["id"].as_i64().unwrap(),
        PHOTO_FIXTURE_BYTES,
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let photo = read_json_body(response).await;
    assert_eq!(photo["original_filename"], "client-name.jpg");
    assert_eq!(photo["url"], format!("/photos/{}", photo["id"]));
    assert_eq!(
        photo["thumbnail_url"],
        format!("/photos/{}/thumbnail", photo["id"])
    );
    assert!(photo.get("storage_key").is_none());
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_can_browse_all_log_photos(pool: PgPool) {
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
    let photo_response = create_photo(
        &application,
        &session_cookie,
        entry["id"].as_i64().unwrap(),
        PHOTO_FIXTURE_BYTES,
    )
    .await;
    let photo = read_json_body(photo_response).await;

    let thumbnail_response = send_json_request(
        &application,
        Method::GET,
        photo["thumbnail_url"].as_str().unwrap(),
        Value::Null,
        Some(&session_cookie),
    )
    .await;
    assert_eq!(thumbnail_response.status(), StatusCode::OK);
    assert_eq!(
        thumbnail_response
            .headers()
            .get(header::CONTENT_TYPE)
            .unwrap(),
        "image/jpeg"
    );
    let thumbnail_bytes = thumbnail_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    let thumbnail = image::load_from_memory(&thumbnail_bytes).unwrap();
    assert!(thumbnail.width() <= 1024);
    assert!(thumbnail.height() <= 1024);

    let photo_response = send_json_request(
        &application,
        Method::GET,
        photo["url"].as_str().unwrap(),
        Value::Null,
        Some(&session_cookie),
    )
    .await;
    assert_eq!(photo_response.status(), StatusCode::OK);
    assert_eq!(
        photo_response.headers().get(header::CONTENT_TYPE).unwrap(),
        "image/jpeg"
    );
    assert_eq!(
        photo_response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes(),
        PHOTO_FIXTURE_BYTES
    );
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn user_cannot_access_another_users_photo(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let owner_cookie = register_and_sign_in(&application, "owner@example.com").await;
    let other_user_cookie = register_and_sign_in(&application, "other@example.com").await;
    let plant = create_plant(&application, &owner_cookie, "Mustard").await;
    let grow_log = create_log(
        &application,
        &owner_cookie,
        plant["id"].as_i64().unwrap(),
        "Spring grow",
    )
    .await;
    let entry = create_entry(
        &application,
        &owner_cookie,
        grow_log["id"].as_i64().unwrap(),
        "2024-03-08T10:30:00Z",
        "seedling",
    )
    .await;
    let photo_response = create_photo(
        &application,
        &owner_cookie,
        entry["id"].as_i64().unwrap(),
        PHOTO_FIXTURE_BYTES,
    )
    .await;
    let photo = read_json_body(photo_response).await;

    let response = send_json_request(
        &application,
        Method::GET,
        &format!("/photos/{}", photo["id"]),
        Value::Null,
        Some(&other_user_cookie),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}
