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

pub async fn read_json_body(response: Response<Body>) -> Value {
    let response_body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&response_body).unwrap()
}

pub fn create_json_request(method: Method, uri: &str, request_body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap()
}

pub async fn send_json_request(
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

pub async fn register_and_sign_in(application: &axum::Router, user_email: &str) -> String {
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

pub async fn create_plant(application: &axum::Router, session_cookie: &str, name: &str) -> Value {
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

pub async fn create_log(
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

pub async fn create_entry(
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

pub fn create_application(pool: PgPool) -> (axum::Router, PathBuf) {
    let photo_storage_directory =
        std::env::temp_dir().join(format!("grow-axum-feature-test-{}", uuid::Uuid::new_v4()));
    (
        build_app_with_photo_storage_directory(pool, photo_storage_directory.clone()),
        photo_storage_directory,
    )
}

pub const PHOTO_FIXTURE_BYTES: &[u8] = include_bytes!("../fixtures/IMG_20261106_174117.jpg");

pub async fn create_photo(
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
    let request = axum::http::Request::post(format!("/entries/{entry_id}/photos"))
        .header(header::COOKIE, session_cookie)
        .header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={multipart_boundary}"),
        )
        .body(Body::from(multipart_body))
        .unwrap();
    application.clone().oneshot(request).await.unwrap()
}
