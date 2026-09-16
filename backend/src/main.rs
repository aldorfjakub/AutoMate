use std::{env, time::Duration};

use crate::{
    handlers::play::start_match,
    models::dto::{BotSummary},
    routes::app_routes,
};
use dotenvy::dotenv;
use redis::{Client, aio::MultiplexedConnection};
use sqlx::SqlitePool;
use tokio::time::sleep;
use uuid::Uuid;

mod db;
mod error;
mod extract;
mod handlers;
mod models;
mod routes;
mod state;

const RANDOM_BOT: &'static str = r#"import chess
import random

number = 1

def get_chess_move(fen: str):
    global number
    """Jednoduchý šachový bot."""
    board = chess.Board(fen)
    legal_moves = list(board.generate_legal_moves())
    captures = list(board.generate_legal_captures())


    # 1. Zkusit dát mat
    for move in legal_moves:
        board.push(move)
        if board.is_checkmate():
            return move.uci()
        board.pop()
        
    # 2. Povýšení na královnu
    for move in legal_moves:
        if "q" in move.uci().lower():
            return move.uci()
            
    # 3. Braní figurky
    if captures:
        return random.choice(captures).uci()
        
    # 4. Náhodný tah
    return random.choice(legal_moves).uci()
"#;

const GREEDY_BOT: &'static str = r#"
import chess
import random

PIECE_VALUES = {
    chess.PAWN: 1, chess.KNIGHT: 3, chess.BISHOP: 3,
    chess.ROOK: 5, chess.QUEEN: 9, chess.KING: 1000
}

def evaluate(board):
    if board.is_checkmate():
        return -9999 if board.turn == chess.WHITE else 9999
    
    score = 0
    for square, piece in board.piece_map().items():
        val = PIECE_VALUES[piece.piece_type]
        score += val if piece.color == chess.WHITE else -val
    return score

def get_chess_move(fen: str) -> str:
    board = chess.Board(fen)
    
    # Handle terminal state
    if board.is_game_over() or not any(board.legal_moves):
        return None

    best_moves = []
    is_white = board.turn == chess.WHITE
    best_score = float('-inf') if is_white else float('inf')

    for move in board.legal_moves:
        board.push(move)
        score = evaluate(board)
        board.pop()

        if (is_white and score > best_score) or (not is_white and score < best_score):
            best_score = score
            best_moves = [move]
        elif score == best_score:
            best_moves.append(move)

    chosen_move = random.choice(best_moves)
    return chosen_move.uci()"#;

// TEMPORARY
async fn add_system_bots(db_pool: SqlitePool) {
    let new_id = Uuid::from_u128(0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128);
    let _ = sqlx::query!("INSERT INTO bots (id, name, description, source_code, is_active, is_public, is_system, is_valid) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", new_id, "Random bot", "All moves are pure random", RANDOM_BOT,true, true, true, true).execute(&db_pool).await;
    let new_id = Uuid::from_u128(0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d6u128);
    let _ = sqlx::query!("INSERT INTO bots (id, name, description, source_code, is_active, is_public, is_system, is_valid) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", new_id, "Greedy bot 2", "Takes if he can", GREEDY_BOT,true, true, true, true).execute(&db_pool).await;
}

async fn schedule_match(
    db_pool: &SqlitePool,
    redis_conn: MultiplexedConnection,
) -> Result<(), String> {
    // Find most suitable bot, then opponetn
    // then schedule it, and write into redis
    let bot = match sqlx::query_as!(BotSummary, r#"SELECT id as "id: uuid::Uuid", user_id as "owner_id: uuid::Uuid", name, description, is_active, is_public, is_valid, rating, total_matches FROM bots WHERE is_active = 1 AND is_valid = 1 AND user_id IS NOT NULL ORDER BY last_played_at ASC LIMIT 1"#)
        .fetch_optional(db_pool)
        .await.map_err(|e| format!("Failed to pull bot from db: {}",e))? {
            Some(b) => b,
            None => return Err("No suitable bot found".to_string())
        };

    // Try with low elo difference then each loop increase the possible difference until opponent is found/no opponent error
    let bot_id = bot.id.ok_or_else(|| "bot has no id".to_string())?;
    let mut opponent: Option<BotSummary> = None;

    for i in 1..6u32 {
        if opponent.is_some() {
            break;
        }
        let rating_min = bot.rating - 150f64 * i as f64;
        let rating_max = bot.rating + 150f64 * i as f64;

        opponent = sqlx::query_as!(BotSummary, r#"SELECT id as "id: uuid::Uuid", user_id as "owner_id: uuid::Uuid", name, description, is_active, is_public, is_valid, rating, total_matches FROM bots WHERE is_active = 1 AND is_valid = 1 AND id != ? AND rating BETWEEN ? AND ? AND id NOT IN (
      SELECT CASE WHEN white_bot_id = ? THEN black_bot_id ELSE white_bot_id END
      FROM matches 
      WHERE white_bot_id = ? OR black_bot_id = ?
      ORDER BY created_at DESC 
      LIMIT 3
  )  AND (user_id IS NULL OR user_id != ?) ORDER BY RANDOM() LIMIT 1"#, &bot_id, rating_min, rating_max, &bot_id, &bot_id, &bot_id, &bot.owner_id).fetch_optional(db_pool).await.map_err(|e| format!("Failed to pull bot from db: {}",e))?;
    }

    let opponent = opponent.ok_or_else(|| "No suitable opponent for bot found".to_string())?;

    start_match(db_pool.clone(), redis_conn, bot, opponent, true)
        .await
        .map_err(|e| format!("{:?}", e))?;


    // TODO entry into redis
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let db_pool = db::db::init_pool().await;
    add_system_bots(db_pool.clone()).await;
    let redis_client = Client::open(env::var("REDIS_URL").expect("REDIS_URL must be set"))
        .expect("Failed to create Redis client");
    let redis_connection = redis_client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis");

    let cleanup_pool = db_pool.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_hours(2)).await;
            let _ = sqlx::query!("DELETE FROM sessions WHERE expires_at < CURRENT_TIMESTAMP")
                .execute(&cleanup_pool)
                .await;
        }
    });

    let schedule_db = db_pool.clone();
    let redis_conn = redis_connection.clone();

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(30)).await;
            let _ = sqlx::query!("UPDATE matches SET match_status = 'failed' WHERE match_status IN (\"pending\", \"playing\") AND completed_at IS NULL AND created_at < datetime('now', '-15 minutes')").execute(&schedule_db).await;

            let res = sqlx::query!("SELECT * FROM matches WHERE is_ranked = 1 AND match_status IN (\"pending\", \"playing\") LIMIT 1").fetch_one(&schedule_db).await;
            if res.is_ok() {
                continue;
            }

            let _ = schedule_match(&schedule_db, redis_conn.clone()).await;
        }
    });

    let app_state = state::AppState {
        sqlite_pool: db_pool.clone(),
        redis_client: redis_client,
        redis_con: redis_connection,
        oauth_client_id: env::var("OAUTH_CLIENT_ID").unwrap(),
        oauth_client_secret: env::var("OAUTH_CLIENT_SECRET").unwrap(),
        oauth_callback_url: env::var("OAUTH_CALLBACK").unwrap(),
    };

    let app = app_routes(app_state);
    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("app running at {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, app).await;
}
