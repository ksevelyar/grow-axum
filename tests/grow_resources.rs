use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use cookie::Cookie;
use grow_axum::build_app_with_photo_storage_directory;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::path::PathBuf;
use tower::ServiceExt;

const PHOTO_FIXTURE_BYTES: &[u8] = include_bytes!("fixtures/IMG_20261106_174117.jpg");

async fn read_json_body(response: Response<Body>) -> Value {
    let response_body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&response_body).unwrap()
}

fn create_json_request(method: Method, uri: &str, request_body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap()
}

async fn send_json_request(
    application: &axum::Router,
    method: Method,
    uri: &str,
    request_body: Value,
    session_cookie: Option<&str>,
) -> Response<Body> {
    let mut request = create_json_request(method, uri, request_body);
    if let Some(session_cookie) = session_cookie {
        request
            .headers_mut()
            .insert(header::COOKIE, session_cookie.parse().unwrap());
    }
    application.clone().oneshot(request).await.unwrap()
}

async fn register_and_sign_in(application: &axum::Router, user_email: &str) -> String {
    let registration_response = application
        .clone()
        .oneshot(create_json_request(
            Method::POST,
            "/users",
            json!({
                "email": user_email,
                "handle": format!("grower-{}", uuid::Uuid::new_v4()),
                "password": "correct horse battery staple",
                "timezone": "Europe/London"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(registration_response.status(), StatusCode::CREATED);
    let session_response = application
        .clone()
        .oneshot(create_json_request(
            Method::POST,
            "/sessions",
            json!({
                "email": user_email,
                "password": "correct horse battery staple"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(session_response.status(), StatusCode::CREATED);
    let set_cookie = session_response.headers().get(header::SET_COOKIE).unwrap();
    let session_cookie = Cookie::parse(set_cookie.to_str().unwrap()).unwrap();
    format!("{}={}", session_cookie.name(), session_cookie.value())
}

async fn create_plant(application: &axum::Router, session_cookie: &str, name: &str) -> Value {
    let response = send_json_request(
        application,
        Method::POST,
        "/plants",
        json!({"name": name}),
        Some(session_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    read_json_body(response).await
}

async fn create_log(
    application: &axum::Router,
    session_cookie: &str,
    plant_id: i64,
    name: &str,
) -> Value {
    let response = send_json_request(
        application,
        Method::POST,
        "/logs",
        json!({
            "plant_id": plant_id,
            "name": name,
            "started_at": "2024-01-01T00:00:00Z",
            "description": null,
            "ended_at": null,
            "outcome": null
        }),
        Some(session_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    read_json_body(response).await
}

async fn create_entry(
    application: &axum::Router,
    session_cookie: &str,
    log_id: i64,
    occurred_at: &str,
    stage: &str,
) -> Value {
    let response = send_json_request(
        application,
        Method::POST,
        &format!("/logs/{log_id}/entries"),
        json!({"occurred_at": occurred_at, "stage": stage}),
        Some(session_cookie),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    read_json_body(response).await
}

async fn create_photo(
    application: &axum::Router,
    session_cookie: &str,
    entry_id: i64,
    file_bytes: &[u8],
) -> Response<Body> {
    let multipart_boundary = "grow-photo-boundary";
    let multipart_headers = format!(
        "--{multipart_boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"client-name.jpg\"\r\nContent-Type: image/jpeg\r\n\r\n"
    );
    let mut multipart_body = multipart_headers.into_bytes();
    multipart_body.extend_from_slice(file_bytes);
    let multipart_closing_boundary = format!("\r\n--{multipart_boundary}--\r\n");
    multipart_body.extend_from_slice(multipart_closing_boundary.as_bytes());
    let request = Request::post(format!("/entries/{entry_id}/photos"))
        .header(header::COOKIE, session_cookie)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={multipart_boundary}"),
        )
        .body(Body::from(multipart_body))
        .unwrap();
    application.clone().oneshot(request).await.unwrap()
}

fn create_application(pool: PgPool) -> (axum::Router, PathBuf) {
    let photo_storage_directory =
        std::env::temp_dir().join(format!("grow-axum-feature-test-{}", uuid::Uuid::new_v4()));
    (
        build_app_with_photo_storage_directory(pool, photo_storage_directory.clone()),
        photo_storage_directory,
    )
}

#[sqlx::test]
async fn plants_are_scoped_unique_and_protected_from_referenced_deletion(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let first_user_cookie =
        register_and_sign_in(&application, "first-plant-owner@example.com").await;
    let second_user_cookie =
        register_and_sign_in(&application, "second-plant-owner@example.com").await;
    let first_plant = create_plant(&application, &first_user_cookie, "Basil").await;
    let duplicate_plant_response = send_json_request(
        &application,
        Method::POST,
        "/plants",
        json!({"name": "Basil"}),
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(
        duplicate_plant_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        read_json_body(duplicate_plant_response).await["fields"][0]["field"],
        "name"
    );

    let first_users_additional_plant = create_plant(&application, &first_user_cookie, "Rose").await;
    let second_users_plant = create_plant(&application, &second_user_cookie, "Basil").await;
    assert_ne!(first_plant["id"], second_users_plant["id"]);
    let plant_list_response = send_json_request(
        &application,
        Method::GET,
        "/plants",
        Value::Null,
        Some(&first_user_cookie),
    )
    .await;
    let plant_list = read_json_body(plant_list_response).await;
    assert_eq!(plant_list.as_array().unwrap().len(), 2);
    let duplicate_rename_response = send_json_request(
        &application,
        Method::PATCH,
        &format!("/plants/{}", first_users_additional_plant["id"]),
        json!({"name": "Basil"}),
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(
        duplicate_rename_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let first_users_log = create_log(
        &application,
        &first_user_cookie,
        first_plant["id"].as_i64().unwrap(),
        "Spring grow",
    )
    .await;
    let second_users_log = create_log(
        &application,
        &second_user_cookie,
        second_users_plant["id"].as_i64().unwrap(),
        "Private grow",
    )
    .await;
    assert_ne!(first_users_log["id"], second_users_log["id"]);

    let plant_detail_response = send_json_request(
        &application,
        Method::GET,
        &format!("/plants/{}", first_plant["id"]),
        Value::Null,
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(plant_detail_response.status(), StatusCode::OK);
    let plant_detail = read_json_body(plant_detail_response).await;
    assert_eq!(plant_detail["grow_logs"].as_array().unwrap().len(), 1);
    assert_eq!(plant_detail["grow_logs"][0]["id"], first_users_log["id"]);
    assert!(plant_detail["grow_logs"][0].get("entries").is_none());

    let cross_user_plant_response = send_json_request(
        &application,
        Method::GET,
        &format!("/plants/{}", second_users_plant["id"]),
        Value::Null,
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(cross_user_plant_response.status(), StatusCode::NOT_FOUND);
    let referenced_plant_delete_response = send_json_request(
        &application,
        Method::DELETE,
        &format!("/plants/{}", first_plant["id"]),
        Value::Null,
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(
        referenced_plant_delete_response.status(),
        StatusCode::CONFLICT
    );

    let rename_response = send_json_request(
        &application,
        Method::PATCH,
        &format!("/plants/{}", first_plant["id"]),
        json!({"name": "Sweet basil"}),
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(rename_response.status(), StatusCode::OK);
    assert_eq!(read_json_body(rename_response).await["name"], "Sweet basil");
    let first_user_logs_response = send_json_request(
        &application,
        Method::GET,
        "/logs",
        Value::Null,
        Some(&first_user_cookie),
    )
    .await;
    let first_user_logs = read_json_body(first_user_logs_response).await;
    assert_eq!(first_user_logs.as_array().unwrap().len(), 1);
    assert_eq!(first_user_logs[0]["id"], first_users_log["id"]);

    let log_delete_response = send_json_request(
        &application,
        Method::DELETE,
        &format!("/logs/{}", first_users_log["id"]),
        Value::Null,
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(log_delete_response.status(), StatusCode::NO_CONTENT);
    let plant_delete_response = send_json_request(
        &application,
        Method::DELETE,
        &format!("/plants/{}", first_plant["id"]),
        Value::Null,
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(plant_delete_response.status(), StatusCode::NO_CONTENT);
    let unreferenced_plant_delete_response = send_json_request(
        &application,
        Method::DELETE,
        &format!("/plants/{}", first_users_additional_plant["id"]),
        Value::Null,
        Some(&first_user_cookie),
    )
    .await;
    assert_eq!(
        unreferenced_plant_delete_response.status(),
        StatusCode::NO_CONTENT
    );
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn logs_validate_ownership_dates_stages_and_return_gallery_and_detail_data(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool.clone());
    let owner_cookie = register_and_sign_in(&application, "log-owner@example.com").await;
    let other_user_cookie = register_and_sign_in(&application, "other-log-owner@example.com").await;
    let owner_plant = create_plant(&application, &owner_cookie, "Tomato").await;
    let other_plant = create_plant(&application, &other_user_cookie, "Pepper").await;
    let owner_plant_id = owner_plant["id"].as_i64().unwrap();
    let other_plant_id = other_plant["id"].as_i64().unwrap();

    let foreign_plant_log_response = send_json_request(
        &application,
        Method::POST,
        "/logs",
        json!({
            "plant_id": other_plant_id,
            "name": "Forbidden",
            "started_at": "2024-01-01T00:00:00Z"
        }),
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(foreign_plant_log_response.status(), StatusCode::NOT_FOUND);
    let invalid_end_date_response = send_json_request(
        &application,
        Method::POST,
        "/logs",
        json!({
            "plant_id": owner_plant_id,
            "name": "Invalid dates",
            "started_at": "2024-01-02T00:00:00Z",
            "ended_at": "2024-01-01T00:00:00Z"
        }),
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(
        invalid_end_date_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        read_json_body(invalid_end_date_response).await["fields"][0]["field"],
        "ended_at"
    );

    let grow_log = create_log(&application, &owner_cookie, owner_plant_id, "Tomato season").await;
    let log_id = grow_log["id"].as_i64().unwrap();
    let other_users_log = create_log(
        &application,
        &other_user_cookie,
        other_plant_id,
        "Other log",
    )
    .await;
    let foreign_log_read_response = send_json_request(
        &application,
        Method::GET,
        &format!("/logs/{}", other_users_log["id"]),
        Value::Null,
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(foreign_log_read_response.status(), StatusCode::NOT_FOUND);
    let foreign_log_update_response = send_json_request(
        &application,
        Method::PATCH,
        &format!("/logs/{log_id}"),
        json!({
            "plant_id": other_plant_id,
            "name": "Moved away",
            "started_at": "2024-01-01T00:00:00Z"
        }),
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(foreign_log_update_response.status(), StatusCode::NOT_FOUND);

    let entry_dates = [
        ("2024-01-01T00:00:00Z", "germination"),
        ("2024-01-02T00:00:00Z", "seedling"),
        ("2024-01-03T00:00:00Z", "vegetation"),
        ("2024-01-04T00:00:00Z", "bloom"),
    ];
    let mut entry_ids = Vec::new();
    for (occurred_at, stage) in entry_dates {
        let entry = create_entry(&application, &owner_cookie, log_id, occurred_at, stage).await;
        entry_ids.push(entry["id"].as_i64().unwrap());
    }
    let invalid_stage_response = send_json_request(
        &application,
        Method::POST,
        &format!("/logs/{log_id}/entries"),
        json!({"occurred_at": "2024-01-05T00:00:00Z", "stage": "flowering"}),
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(
        invalid_stage_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        read_json_body(invalid_stage_response).await["fields"][0]["field"],
        "stage"
    );
    let newest_entry_id = *entry_ids.last().unwrap();
    let uploaded_photo_response = create_photo(
        &application,
        &owner_cookie,
        newest_entry_id,
        PHOTO_FIXTURE_BYTES,
    )
    .await;
    assert_eq!(uploaded_photo_response.status(), StatusCode::CREATED);
    let uploaded_photo = read_json_body(uploaded_photo_response).await;
    let photo_id = uploaded_photo["id"].as_i64().unwrap();
    let photo_storage_key = sqlx::query_scalar::<_, String>(
        "SELECT storage_key FROM grow_log_entry_photos WHERE id = $1",
    )
    .bind(photo_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let photo_path = photo_storage_directory.join(photo_storage_key);
    assert!(tokio::fs::try_exists(&photo_path).await.unwrap());

    let gallery_response = send_json_request(
        &application,
        Method::GET,
        "/logs",
        Value::Null,
        Some(&owner_cookie),
    )
    .await;
    let gallery_logs = read_json_body(gallery_response).await;
    assert_eq!(gallery_logs.as_array().unwrap().len(), 1);
    assert_eq!(gallery_logs[0]["newest_entry_photo_id"], photo_id);
    let log_detail_response = send_json_request(
        &application,
        Method::GET,
        &format!("/logs/{log_id}"),
        Value::Null,
        Some(&owner_cookie),
    )
    .await;
    let log_detail = read_json_body(log_detail_response).await;
    let entry_stages: Vec<&str> = log_detail["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["stage"].as_str().unwrap())
        .collect();
    assert_eq!(
        entry_stages,
        ["bloom", "vegetation", "seedling", "germination"]
    );
    assert_eq!(log_detail["entries"][0]["photos"][0]["id"], photo_id);

    let photo_access_by_other_user = send_json_request(
        &application,
        Method::GET,
        &format!("/photos/{photo_id}"),
        Value::Null,
        Some(&other_user_cookie),
    )
    .await;
    assert_eq!(photo_access_by_other_user.status(), StatusCode::NOT_FOUND);
    let photo_response = send_json_request(
        &application,
        Method::GET,
        &format!("/photos/{photo_id}"),
        Value::Null,
        Some(&owner_cookie),
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

    let valid_log_update_response = send_json_request(
        &application,
        Method::PATCH,
        &format!("/logs/{log_id}"),
        json!({
            "plant_id": owner_plant_id,
            "name": "Completed tomato season",
            "started_at": "2024-01-01T00:00:00Z",
            "ended_at": "2024-02-01T00:00:00Z",
            "outcome": "Harvested"
        }),
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(valid_log_update_response.status(), StatusCode::OK);
    assert_eq!(
        read_json_body(valid_log_update_response).await["name"],
        "Completed tomato season"
    );
    let delete_log_response = send_json_request(
        &application,
        Method::DELETE,
        &format!("/logs/{log_id}"),
        Value::Null,
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(delete_log_response.status(), StatusCode::NO_CONTENT);
    assert!(!tokio::fs::try_exists(photo_path).await.unwrap());
    let remaining_photo_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM grow_log_entry_photos WHERE id = $1")
            .bind(photo_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(remaining_photo_count, 0);
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn photo_and_entry_deletion_remove_files_and_reject_foreign_users(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool.clone());
    let owner_cookie = register_and_sign_in(&application, "entry-owner@example.com").await;
    let other_user_cookie = register_and_sign_in(&application, "entry-outsider@example.com").await;
    let owner_plant = create_plant(&application, &owner_cookie, "Mint").await;
    let other_plant = create_plant(&application, &other_user_cookie, "Parsley").await;
    let owner_log = create_log(
        &application,
        &owner_cookie,
        owner_plant["id"].as_i64().unwrap(),
        "Mint log",
    )
    .await;
    let owner_log_id = owner_log["id"].as_i64().unwrap();
    let other_log = create_log(
        &application,
        &other_user_cookie,
        other_plant["id"].as_i64().unwrap(),
        "Parsley log",
    )
    .await;
    let other_entry = create_entry(
        &application,
        &other_user_cookie,
        other_log["id"].as_i64().unwrap(),
        "2024-03-01T00:00:00Z",
        "seedling",
    )
    .await;
    let foreign_entry_id = other_entry["id"].as_i64().unwrap();
    let owner_entry = create_entry(
        &application,
        &owner_cookie,
        owner_log_id,
        "2024-03-02T00:00:00Z",
        "vegetation",
    )
    .await;
    let owner_entry_id = owner_entry["id"].as_i64().unwrap();

    let foreign_entry_update = send_json_request(
        &application,
        Method::PATCH,
        &format!("/entries/{foreign_entry_id}"),
        json!({"occurred_at": "2024-03-03T00:00:00Z", "stage": "bloom"}),
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(foreign_entry_update.status(), StatusCode::NOT_FOUND);
    let foreign_entry_delete = send_json_request(
        &application,
        Method::DELETE,
        &format!("/entries/{foreign_entry_id}"),
        Value::Null,
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(foreign_entry_delete.status(), StatusCode::NOT_FOUND);
    let foreign_entry_photo_upload = create_photo(
        &application,
        &owner_cookie,
        foreign_entry_id,
        PHOTO_FIXTURE_BYTES,
    )
    .await;
    assert_eq!(foreign_entry_photo_upload.status(), StatusCode::NOT_FOUND);

    let photo_response = create_photo(
        &application,
        &owner_cookie,
        owner_entry_id,
        PHOTO_FIXTURE_BYTES,
    )
    .await;
    assert_eq!(photo_response.status(), StatusCode::CREATED);
    let photo = read_json_body(photo_response).await;
    assert_eq!(photo["original_filename"], "client-name.jpg");
    assert!(photo.get("storage_key").is_none());
    let photo_id = photo["id"].as_i64().unwrap();
    let photo_storage_key = sqlx::query_scalar::<_, String>(
        "SELECT storage_key FROM grow_log_entry_photos WHERE id = $1",
    )
    .bind(photo_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let photo_path = photo_storage_directory.join(photo_storage_key);
    assert!(tokio::fs::try_exists(&photo_path).await.unwrap());
    let foreign_photo_delete = send_json_request(
        &application,
        Method::DELETE,
        &format!("/photos/{photo_id}"),
        Value::Null,
        Some(&other_user_cookie),
    )
    .await;
    assert_eq!(foreign_photo_delete.status(), StatusCode::NOT_FOUND);
    tokio::fs::remove_file(&photo_path).await.unwrap();
    let photo_delete = send_json_request(
        &application,
        Method::DELETE,
        &format!("/photos/{photo_id}"),
        Value::Null,
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(photo_delete.status(), StatusCode::NO_CONTENT);

    let entry_photo_response = create_photo(
        &application,
        &owner_cookie,
        owner_entry_id,
        PHOTO_FIXTURE_BYTES,
    )
    .await;
    let entry_photo = read_json_body(entry_photo_response).await;
    let entry_photo_storage_key = sqlx::query_scalar::<_, String>(
        "SELECT storage_key FROM grow_log_entry_photos WHERE id = $1",
    )
    .bind(entry_photo["id"].as_i64().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    let entry_photo_path = photo_storage_directory.join(entry_photo_storage_key);
    let entry_delete = send_json_request(
        &application,
        Method::DELETE,
        &format!("/entries/{owner_entry_id}"),
        Value::Null,
        Some(&owner_cookie),
    )
    .await;
    assert_eq!(entry_delete.status(), StatusCode::NO_CONTENT);
    assert!(!tokio::fs::try_exists(entry_photo_path).await.unwrap());
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}

#[sqlx::test]
async fn browser_api_preflight_allows_credentials_from_the_frontend_origin(pool: PgPool) {
    let (application, photo_storage_directory) = create_application(pool);
    let preflight_request = Request::builder()
        .method(Method::OPTIONS)
        .uri("/logs")
        .header(header::ORIGIN, "http://grow.localhost:3001")
        .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
        .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "content-type")
        .body(Body::empty())
        .unwrap();

    let preflight_response = application.oneshot(preflight_request).await.unwrap();

    assert_eq!(preflight_response.status(), StatusCode::OK);
    assert_eq!(
        preflight_response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap(),
        "http://grow.localhost:3001"
    );
    assert_eq!(
        preflight_response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS)
            .unwrap(),
        "true"
    );
    tokio::fs::remove_dir_all(photo_storage_directory)
        .await
        .unwrap_or(());
}
