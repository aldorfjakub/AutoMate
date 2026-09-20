use axum::Router;
use std::sync::Arc;

use crate::{
    handlers::{
        auth_routes, bots_routes, job_routes, leaderboard_routes, matches_routes, play_routes,
        user_routes,
    },
    state::AppState,
};

fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .nest("/auth", auth_routes())
        .nest("/user", user_routes())
        .nest("/bots", bots_routes())
        .nest("/jobs", job_routes())
        .nest("/play", play_routes())
        .nest("/matches", matches_routes())
        .nest("/leaderboard", leaderboard_routes())
}

pub fn app_routes(state: AppState) -> Router<()> {
    let state = Arc::new(state);
    let api_routes = Router::new().nest("/api", api_routes()).with_state(state);
    // .layer(middleware::from_fn(auth))
    // .layer(middleware::from_fn(advanced_logging))

    api_routes
}
