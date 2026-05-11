use axum::Router;
use tower_http::trace::TraceLayer;

pub async fn app() -> anyhow::Result<Router> {
    let app = Router::new()
        .merge(auth_routes())
        .layer(TraceLayer::new_for_http());

    Ok(app)
}

fn auth_routes() -> Router {
    Router::new()
        .route("/auth/login", axum::routing::post(login))
}

async fn login() -> &'static str {
    "Login endpoint"
}
