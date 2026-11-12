use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use cookie::Cookie;
use grow_axum::build_app;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::{Service, ServiceExt};

async fn read_json_body(response: Response<Body>) -> Value {
    let response_body = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&response_body).unwrap()
}

fn create_json_request(uri: &str, request_body: Value) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap()
}

fn extract_session_cookie(response: &Response<Body>) -> String {
    let set_cookie_header = response.headers().get(header::SET_COOKIE).unwrap();
    let parsed_cookie = Cookie::parse(set_cookie_header.to_str().unwrap()).unwrap();
    assert_eq!(parsed_cookie.name(), "jwt");
    assert!(parsed_cookie.http_only().unwrap());
    assert_eq!(parsed_cookie.path(), Some("/"));
    format!("{}={}", parsed_cookie.name(), parsed_cookie.value())
}

fn create_registration_payload() -> Value {
    json!({
        "email": "person@example.com",
        "handle": "person",
        "password": "correct horse battery staple",
        "timezone": "Europe/London"
    })
}

async fn register_user(application: &mut axum::routing::RouterIntoService<Body>) -> Value {
    let registration_response = application
        .ready()
        .await
        .unwrap()
        .call(create_json_request("/users", create_registration_payload()))
        .await
        .unwrap();
    assert_eq!(registration_response.status(), StatusCode::CREATED);
    read_json_body(registration_response).await
}

#[sqlx::test]
async fn user_registers_an_account(pool: PgPool) {
    let mut application = build_app(pool).into_service();

    let registered_user = register_user(&mut application).await;

    assert!(registered_user["id"].is_i64());
    assert_eq!(registered_user["email"], "person@example.com");
    assert_eq!(registered_user["timezone"], "Europe/London");
    assert!(registered_user.get("password_hash").is_none());
}

#[sqlx::test]
async fn user_cannot_register_an_account_with_an_email_already_in_use(pool: PgPool) {
    let mut application = build_app(pool).into_service();
    register_user(&mut application).await;

    let duplicate_user = json!({
        "email": "PERSON@example.com",
        "handle": "another-person",
        "password": "correct horse battery staple",
        "timezone": "Europe/London"
    });
    let duplicate_response = application
        .ready()
        .await
        .unwrap()
        .call(create_json_request("/users", duplicate_user))
        .await
        .unwrap();

    assert_eq!(
        duplicate_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        read_json_body(duplicate_response).await,
        json!({"error": "validation failed", "fields": [{"field": "email", "message": "email is already taken"}]})
    );
}

#[sqlx::test]
async fn user_cannot_register_an_account_with_an_unknown_timezone(pool: PgPool) {
    let mut application = build_app(pool).into_service();
    let invalid_timezone_user = json!({
        "email": "person@example.com",
        "handle": "person",
        "password": "correct horse battery staple",
        "timezone": "Mars/Olympus_Mons"
    });

    let registration_response = application
        .ready()
        .await
        .unwrap()
        .call(create_json_request("/users", invalid_timezone_user))
        .await
        .unwrap();

    assert_eq!(
        registration_response.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        read_json_body(registration_response).await,
        json!({"error": "validation failed", "fields": [{"field": "timezone", "message": "unknown timezone"}]})
    );
}

#[sqlx::test]
async fn user_signs_in(pool: PgPool) {
    let mut application = build_app(pool).into_service();
    register_user(&mut application).await;

    let session_response = application
        .ready()
        .await
        .unwrap()
        .call(create_json_request(
            "/sessions",
            json!({"email": "person@example.com", "password": "correct horse battery staple"}),
        ))
        .await
        .unwrap();

    assert_eq!(session_response.status(), StatusCode::CREATED);
    extract_session_cookie(&session_response);
}

#[sqlx::test]
async fn user_cannot_sign_in_with_invalid_credentials(pool: PgPool) {
    let mut application = build_app(pool).into_service();
    register_user(&mut application).await;

    let session_response = application
        .ready()
        .await
        .unwrap()
        .call(create_json_request(
            "/sessions",
            json!({"email": "person@example.com", "password": "incorrect password"}),
        ))
        .await
        .unwrap();

    assert_eq!(session_response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        read_json_body(session_response).await,
        json!({"error": "unauthorized"})
    );
}

#[sqlx::test]
async fn user_cannot_see_the_signed_in_user_without_a_session(pool: PgPool) {
    let application = build_app(pool).into_service();

    let current_user_response = application
        .oneshot(
            Request::get("/sessions/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(current_user_response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        read_json_body(current_user_response).await,
        json!({"error": "unauthorized"})
    );
}

#[sqlx::test]
async fn user_sees_the_signed_in_user(pool: PgPool) {
    let mut application = build_app(pool).into_service();
    let registered_user = register_user(&mut application).await;
    let session_response = application
        .ready()
        .await
        .unwrap()
        .call(create_json_request(
            "/sessions",
            json!({"email": "person@example.com", "password": "correct horse battery staple"}),
        ))
        .await
        .unwrap();
    let session_cookie = extract_session_cookie(&session_response);

    let current_user_request = Request::get("/sessions/current")
        .header(header::COOKIE, session_cookie)
        .body(Body::empty())
        .unwrap();
    let current_user_response = application
        .ready()
        .await
        .unwrap()
        .call(current_user_request)
        .await
        .unwrap();

    assert_eq!(current_user_response.status(), StatusCode::OK);
    assert_eq!(read_json_body(current_user_response).await, registered_user);
}
