use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::{routing::get, Router};
use tower::util::ServiceExt;

fn test_app() -> Router {
    Router::new().route("/", get(|| async { "Hello" }))
}

#[tokio::test]
async fn hello_world() {
    let app = test_app();

    let resp = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(&body[..], b"Hello");
}
