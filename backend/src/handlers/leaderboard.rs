use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::AppResult, state::AppState};

const DEFAULT_PAGE_SIZE: i64 = 20;
const MAX_PAGE_SIZE: i64 = 100;
const MIN_PAGE: i64 = 1;

#[derive(Deserialize)]
pub struct LeaderboardQuery {
    page: Option<i64>,
    page_size: Option<i64>,
}

#[derive(Serialize)]
pub struct LeaderboardEntry {
    rank: i64,
    bot_id: Uuid,
    name: String,
    rating: f64,
    total_matches: i64,
    author: Option<String>,
}

#[derive(Serialize)]
pub struct LeaderboardResponse {
    items: Vec<LeaderboardEntry>,
    page: i64,
    page_size: i64,
    total: i64,
    total_pages: i64,
}

async fn get_leaderboard(
    State(state): State<Arc<AppState>>,
    Query(query): Query<LeaderboardQuery>,
) -> AppResult<Json<LeaderboardResponse>> {
    let page = query.page.unwrap_or(MIN_PAGE).max(MIN_PAGE);
    let page_size = query
        .page_size
        .unwrap_or(DEFAULT_PAGE_SIZE)
        .clamp(1, MAX_PAGE_SIZE);
    let offset = (page - 1) * page_size;

    let total: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM bots WHERE total_matches > 0 AND is_active = 1")
            .fetch_one(&state.sqlite_pool)
            .await?;
    let total_pages = (total + page_size - 1) / page_size;

    let items = sqlx::query_as!(
        LeaderboardEntry,
        r#"
        SELECT
            ROW_NUMBER() OVER (
                ORDER BY b.rating DESC, b.total_matches DESC, b.created_at ASC
            ) AS "rank!: i64",
            b.id AS "bot_id!: Uuid",
            b.name AS "name!: String",
            b.rating AS "rating!: f64",
            b.total_matches AS "total_matches!: i64",
            u.display_name AS "author"
        FROM bots b
        LEFT JOIN users u ON u.id = b.user_id
        WHERE b.total_matches > 0 AND is_active = 1
        ORDER BY b.rating DESC, b.total_matches DESC, b.created_at ASC
        LIMIT ? OFFSET ?
        "#,
        page_size,
        offset,
    )
    .fetch_all(&state.sqlite_pool)
    .await?;

    Ok(Json(LeaderboardResponse {
        items,
        page,
        page_size,
        total,
        total_pages,
    }))
}

pub fn leaderboard_routes() -> Router<Arc<AppState>> {
    Router::new().route("/", get(get_leaderboard))
}
