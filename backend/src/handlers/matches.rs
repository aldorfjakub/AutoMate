use std::{convert::Infallible, sync::Arc, time::Duration};

use axum::{
    Json, Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};

use uuid::Uuid;

use axum::response::sse::{Event, KeepAlive, Sse};
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

use crate::error::{AppError, AppResult};
use crate::extract::SessionUser;
use crate::models::dto::{BotSummary, MatchWithBots, RankedMatch};
use crate::models::matches::Match;
use crate::state::AppState;

async fn get_matches(
    State(state): State<Arc<AppState>>,
    session: SessionUser,
) -> AppResult<impl IntoResponse> {
    let matches: Vec<MatchWithBots> = sqlx::query_as!(
        MatchWithBots,
        r#"
        SELECT
            m.id AS "id: uuid::Uuid",
            m.white_bot_id AS "white_bot_id: Uuid",
            m.black_bot_id AS "black_bot_id: Uuid",
            m.match_status,
            m.is_ranked,
            m.winner_color,
            m.win_reason,
            m.pgn,
            m.white_elo_change,
            m.black_elo_change,
            m.error_message,
            m.created_at,
            m.completed_at,
            w.name AS white_name,
            b.name AS black_name,
            w.rating AS white_rating,
            b.rating AS black_rating
        FROM matches m
        LEFT JOIN bots w ON w.id = m.white_bot_id
        LEFT JOIN bots b ON b.id = m.black_bot_id
        WHERE EXISTS (
            SELECT 1 
            FROM bots owned
            WHERE owned.user_id = ?
            AND (owned.id = m.white_bot_id OR owned.id = m.black_bot_id)
        )
        ORDER BY m.created_at DESC
"#,
        &session.user_id
    )
    .fetch_all(&state.sqlite_pool)
    .await?;

    Ok(Json(matches))
}

async fn get_ranked_match(State(state): State<Arc<AppState>>) -> AppResult<impl IntoResponse> {
    let ranked_match = match sqlx::query_as!(
        Match,
        r#"SELECT  m.id AS "id: uuid::Uuid",
            m.white_bot_id AS "white_bot_id: Uuid",
            m.black_bot_id AS "black_bot_id: Uuid",
            m.match_status,
            m.is_ranked,
            m.winner_color,
            m.win_reason,
            m.pgn,
            m.white_elo_change,
            m.black_elo_change,
            m.error_message,
            m.created_at,
            m.completed_at
        FROM matches m WHERE is_ranked = 1 AND match_status IN ("pending", "playing") LIMIT 1"#
    )
    .fetch_optional(&state.sqlite_pool)
    .await?
    {
        Some(m) => m,
        None => return Err(AppError::NotFound),
    };

    let white_bot = sqlx::query_as!(BotSummary, r#"SELECT id as "id: uuid::Uuid", user_id as "owner_id: uuid::Uuid", name, description, is_active, is_public, is_valid, rating, total_matches FROM bots WHERE id = ?"#, &ranked_match.white_bot_id).fetch_optional(&state.sqlite_pool).await?;
    let black_bot = sqlx::query_as!(BotSummary, r#"SELECT id as "id: uuid::Uuid", user_id as "owner_id: uuid::Uuid", name, description, is_active, is_public, is_valid, rating, total_matches FROM bots WHERE id = ?"#, &ranked_match.black_bot_id).fetch_optional(&state.sqlite_pool).await?;

    let (white_name, white_rating) = match white_bot {
        Some(b) => (b.name, b.rating),
        None => ("Unknown".to_string(), 0f64),
    };
    let (black_name, black_rating) = match black_bot {
        Some(b) => (b.name, b.rating),
        None => ("Unknown".to_string(), 0f64),
    };

    Ok(Json(RankedMatch {
        m: ranked_match,
        white_name,
        black_name,
        white_rating,
        black_rating,
    }))
}

async fn get_match_result(
    State(state): State<Arc<AppState>>,
    Path(match_id): Path<Uuid>,
    _session: SessionUser,
) -> AppResult<Json<Match>> {
    let m = sqlx::query_as!(
        Match,
        r#"
        SELECT id as "id: uuid::Uuid",
            white_bot_id as "white_bot_id: Uuid",
            black_bot_id as "black_bot_id: Uuid",
               match_status, is_ranked, winner_color, win_reason, pgn,
               white_elo_change, black_elo_change, error_message,
               created_at, completed_at
        FROM matches
        WHERE id = ?
    "#,
        &match_id
    )
    .fetch_optional(&state.sqlite_pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(m))
}

async fn watch_match(
    State(state): State<Arc<AppState>>,
    Path(match_id): Path<Uuid>,
    session: SessionUser,
) -> AppResult<impl IntoResponse> {
    let (tx, rx) = mpsc::channel::<Result<Event, Infallible>>(32);

    let mut pubsub = state
        .redis_client
        .get_async_pubsub()
        .await
        .map_err(|_| AppError::Internal("redis pubsub failed"))?;
    pubsub
        .subscribe(format!("matchStream_{match_id}"))
        .await
        .map_err(|_| AppError::Internal("subscribe failed"))?;

    tokio::spawn(async move {
        let mut stream = pubsub.on_message();
        while let Some(msg) = stream.next().await {
            // Receive MatchEvent json
            if tx
                .send(Ok(Event::default()
                    .event("match")
                    .data(msg.get_payload().unwrap_or("".to_string()))))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx))
        .keep_alive(KeepAlive::default().interval(Duration::from_secs(15))))
}

pub fn matches_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_matches))
        .route("/{match_id}", get(get_match_result))
        .route("/{match_id}/watch", get(watch_match))
        .route("/ranked", get(get_ranked_match))
}
