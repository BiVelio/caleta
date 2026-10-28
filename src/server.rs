use crate::audit::ExecutionRecord;
use crate::backend::{Backend, BackendConfig};
use crate::manifest::LoadedManifest;
use crate::verify::verify_manifest;
use anyhow::{bail, Context, Result};
use axum::body::{to_bytes, Body};
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use tower_http::cors::{AllowOrigin, CorsLayer};

const MAX_REQUEST_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub listen: SocketAddr,
    pub api_key: Option<String>,
    pub cors_origins: Vec<String>,
    pub startup_timeout: Duration,
    pub record_out: Option<PathBuf>,
}

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    upstream: Arc<str>,
    upstream_key: Arc<str>,
    public_key: Option<Arc<str>>,
}

pub async fn serve(loaded: LoadedManifest, options: ServeOptions) -> Result<()> {
    validate_exposure(&options)?;
    let verification = verify_manifest(&loaded).await?;
    if !verification.verified {
        bail!("model or backend integrity verification failed; run `caleta verify`");
    }

    let backend_version = Backend::version(&loaded.backend_path()).await;
    let mut backend = Backend::spawn(
        &loaded,
        BackendConfig {
            startup_timeout: options.startup_timeout,
        },
    )
    .await?;
    tracing::info!(
        model = %loaded.manifest.id,
        startup_ms = backend.started_in.as_millis(),
        "local backend ready"
    );

    if let Some(path) = &options.record_out {
        let record = ExecutionRecord::new(
            &loaded,
            &verification,
            options.listen,
            options.api_key.is_some(),
            options.cors_origins.clone(),
            backend_version,
        );
        record.write(path).await?;
    }

    let state = AppState {
        client: reqwest::Client::builder().no_proxy().build()?,
        upstream: backend.base_url.clone().into(),
        upstream_key: backend.api_key.clone().into(),
        public_key: options.api_key.clone().map(Into::into),
    };
    let app = build_app(state, &options.cors_origins)?;
    let listener = tokio::net::TcpListener::bind(options.listen)
        .await
        .with_context(|| format!("cannot bind {}", options.listen))?;
    tracing::info!(listen = %options.listen, "Caleta is ready; prompts are not logged");

    let result = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await;
    backend.shutdown().await.ok();
    result.context("HTTP server failed")
}

fn validate_exposure(options: &ServeOptions) -> Result<()> {
    if !options.listen.ip().is_loopback() && options.api_key.is_none() {
        bail!("refusing a non-loopback listener without CALETA_API_KEY");
    }
    if let Some(key) = &options.api_key {
        if key.len() < 16 {
            bail!("API key must contain at least 16 characters");
        }
    }
    if options.cors_origins.iter().any(|origin| origin == "*") {
        bail!("wildcard CORS is not supported; list explicit origins");
    }
    Ok(())
}

fn build_app(state: AppState, origins: &[String]) -> Result<Router> {
    let protected = Router::new()
        .route("/v1/models", get(proxy))
        .route("/v1/chat/completions", post(proxy))
        .route_layer(middleware::from_fn_with_state(state.clone(), authenticate));
    let mut app = Router::new()
        .route("/health", get(health))
        .merge(protected)
        .with_state(state);

    if !origins.is_empty() {
        let values = origins
            .iter()
            .map(|origin| {
                origin
                    .parse::<HeaderValue>()
                    .with_context(|| format!("invalid CORS origin {origin:?}"))
            })
            .collect::<Result<Vec<_>>>()?;
        app = app.layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list(values))
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]),
        );
    }
    Ok(app)
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({"status":"ok","service":"caleta","version":crate::VERSION}))
}

async fn authenticate(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let Some(expected) = &state.public_key else {
        return next.run(request).await;
    };
    let provided = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if provided
        .map(|value| bool::from(value.as_bytes().ct_eq(expected.as_bytes())))
        .unwrap_or(false)
    {
        next.run(request).await
    } else {
        error_response(StatusCode::UNAUTHORIZED, "invalid or missing API key")
    }
}

async fn proxy(State(state): State<AppState>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let body = match to_bytes(body, MAX_REQUEST_BYTES).await {
        Ok(body) => body,
        Err(_) => return error_response(StatusCode::PAYLOAD_TOO_LARGE, "request body too large"),
    };
    let mut url = format!("{}{}", state.upstream, parts.uri.path());
    if let Some(query) = parts.uri.query() {
        url.push('?');
        url.push_str(query);
    }
    let mut outgoing = state
        .client
        .request(parts.method.clone(), url)
        .bearer_auth(state.upstream_key.as_ref())
        .body(body);
    for (name, value) in &parts.headers {
        if !forward_request_header(name) {
            continue;
        }
        outgoing = outgoing.header(name, value);
    }

    let upstream = match outgoing.send().await {
        Ok(response) => response,
        Err(error) => {
            tracing::warn!(error = %error, "upstream request failed");
            return error_response(StatusCode::BAD_GATEWAY, "local backend request failed");
        }
    };
    let status = upstream.status();
    let headers = upstream.headers().clone();
    let stream = upstream
        .bytes_stream()
        .map(|result| result.map_err(std::io::Error::other));
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = status;
    for (name, value) in &headers {
        if forward_response_header(name) {
            response.headers_mut().append(name, value.clone());
        }
    }
    response
}

fn forward_request_header(name: &header::HeaderName) -> bool {
    !matches!(
        *name,
        header::HOST
            | header::AUTHORIZATION
            | header::CONTENT_LENGTH
            | header::CONNECTION
            | header::TRANSFER_ENCODING
    )
}

fn forward_response_header(name: &header::HeaderName) -> bool {
    !matches!(
        *name,
        header::CONNECTION
            | header::CONTENT_LENGTH
            | header::TRANSFER_ENCODING
            | header::ACCESS_CONTROL_ALLOW_ORIGIN
            | header::ACCESS_CONTROL_ALLOW_CREDENTIALS
            | header::ACCESS_CONTROL_ALLOW_METHODS
            | header::ACCESS_CONTROL_ALLOW_HEADERS
            | header::ACCESS_CONTROL_EXPOSE_HEADERS
            | header::ACCESS_CONTROL_MAX_AGE
    )
}

fn error_response(status: StatusCode, message: &str) -> Response {
    let body = json!({"error":{"message":message,"type":"caleta_error"}});
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

async fn shutdown_signal() {
    if tokio::signal::ctrl_c().await.is_ok() {
        tracing::info!("shutdown requested");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use axum::http::HeaderMap;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    async fn mock_upstream() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let app = Router::new()
            .route(
                "/v1/models",
                get(|headers: HeaderMap| async move {
                    if headers
                        .get(header::AUTHORIZATION)
                        .and_then(|v| v.to_str().ok())
                        != Some("Bearer internal-secret")
                    {
                        return StatusCode::UNAUTHORIZED.into_response();
                    }
                    Json(json!({"object":"list","data":[{"id":"mock"}]})).into_response()
                }),
            )
            .route(
                "/v1/chat/completions",
                post(|| async {
                    Response::builder()
                        .header(header::CONTENT_TYPE, "text/event-stream")
                        .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
                        .body(Body::from(
                            "data: {\"choices\":[{\"delta\":{\"content\":\"hola\"}}]}\n\ndata: [DONE]\n\n",
                        ))
                        .unwrap()
                }),
            );
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        format!("http://{address}")
    }

    fn state(upstream: String, public_key: Option<&str>) -> AppState {
        AppState {
            client: reqwest::Client::builder().no_proxy().build().unwrap(),
            upstream: upstream.into(),
            upstream_key: "internal-secret".into(),
            public_key: public_key.map(|value| Arc::from(value.to_string())),
        }
    }

    #[tokio::test]
    async fn rejects_missing_public_credentials() {
        let app = build_app(
            state(mock_upstream().await, Some("a-secure-public-key")),
            &[],
        )
        .unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn forwards_models_with_internal_credentials() {
        let app = build_app(
            state(mock_upstream().await, Some("a-secure-public-key")),
            &[],
        )
        .unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/v1/models")
                    .header(header::AUTHORIZATION, "Bearer a-secure-public-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert!(String::from_utf8_lossy(&body).contains("mock"));
    }

    #[tokio::test]
    async fn preserves_sse_stream() {
        let app = build_app(state(mock_upstream().await, None), &[]).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/v1/chat/completions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(Bytes::from_static(br#"{"stream":true}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
        assert!(response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .is_none());
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let text = String::from_utf8_lossy(&body);
        assert!(text.contains("hola"));
        assert!(text.contains("[DONE]"));
    }

    #[test]
    fn non_loopback_requires_key() {
        let options = ServeOptions {
            listen: "0.0.0.0:8080".parse().unwrap(),
            api_key: None,
            cors_origins: vec![],
            startup_timeout: Duration::from_secs(1),
            record_out: None,
        };
        assert!(validate_exposure(&options).is_err());
    }
}
