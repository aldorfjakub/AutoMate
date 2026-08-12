use std::sync::Arc;
use axum::Router;

use crate::{handlers::{auth_routes, user_routes}, state::AppState};


fn api_routes() -> Router<Arc<AppState>>{
    Router::new().nest("/auth", auth_routes()).nest("/user", user_routes())
}

pub fn app_routes(state: AppState) -> Router<()> {
    let state = Arc::new(state);
    let api_routes = Router::new()
        .nest("/api", api_routes() )
        .with_state(state);
    // .layer(middleware::from_fn(auth))
    // .layer(middleware::from_fn(advanced_logging))

    api_routes
}
