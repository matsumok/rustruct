use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use rustruct_calc::story_drift_ratios;
use rustruct_types::SeismicModel;
use serde::{Deserialize, Serialize};

async fn health() -> &'static str {
    "OK"
}

#[derive(Deserialize, Serialize)]
struct CalcRequest {
    model: SeismicModel,
    relative_displacements: Vec<f64>,
}

async fn calc(Json(req): Json<CalcRequest>) -> Json<Vec<f64>> {
    let ratios = story_drift_ratios(&req.relative_displacements, &req.model);
    Json(ratios)
}

#[derive(Clone)]
struct AppState {
    pool: sqlx::SqlitePool,
}

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/calc", post(calc))
        .route("/models", post(create_model).get(list_models))
        .with_state(state)
}

async fn create_model(
    State(state): State<AppState>,
    Json(model): Json<SeismicModel>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query("INSERT INTO models(name, data) VALUES (?, ?)")
        .bind(&model.name)
        .bind(serde_json::to_string(&model).unwrap())
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

async fn list_models(State(state): State<AppState>) -> Result<Json<Vec<SeismicModel>>, StatusCode> {
    let models: Vec<String> = sqlx::query_scalar("SELECT data FROM models")
        .fetch_all(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let result = models
        .iter()
        .map(|m| serde_json::from_str(&m).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(result))
}

#[tokio::main]
async fn main() {
    let pool = sqlx::SqlitePool::connect("sqlite:rustruct.db?mode=rwc")
        .await
        .unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    axum::serve(listener, app(AppState { pool })).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_return_ok() {
        let response = app(AppState {
            pool: test_state().await,
        })
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(&body[..], b"OK");
    }

    fn sample_model() -> SeismicModel {
        SeismicModel {
            name: "Sample".to_string(),
            story_masses: vec![1.0, 2.0, 3.0],
            story_stiffnesses: vec![4.0, 5.0, 6.0],
            story_heights: vec![7.0, 8.0, 9.0],
        }
    }

    fn sample_request() -> CalcRequest {
        CalcRequest {
            model: sample_model(),
            relative_displacements: vec![7.0, 8.0, 9.0],
        }
    }

    async fn test_state() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn calc_returns_ok() {
        let req = sample_request();
        let response = app(AppState {
            pool: test_state().await,
        })
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/calc")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_string(&req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result = serde_json::from_slice::<Vec<f64>>(&body).unwrap();
        assert_eq!(result, [1.0, 1.0, 1.0]);
    }

    #[tokio::test]
    async fn calc_missing_field_returns_422() {
        let broken = r#"{"model":{"name":"S","story_masses":[1.0],"story_stiffnesses":[1.0]},"relative_displacements":[1.0]}"#;
        let response = app(AppState {
            pool: test_state().await,
        })
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/calc")
                .header("Content-Type", "application/json")
                .body(Body::from(broken))
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn calc_unsupported_media_type_returns_415() {
        let req = sample_request();
        let response = app(AppState {
            pool: test_state().await,
        })
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/calc")
                .body(Body::from(serde_json::to_string(&req).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn create_then_list_models() {
        let req = sample_model();
        let app = app(AppState {
            pool: test_state().await,
        });
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/models")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&req).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/models")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let result = serde_json::from_slice::<Vec<SeismicModel>>(&body).unwrap();

        assert_eq!(result, [req])
    }
}
