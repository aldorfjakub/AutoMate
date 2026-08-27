use std::{env, time::Duration};

use crate::routes::app_routes;
use dotenvy::dotenv;
use redis::Client;
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

def get_best_move(board):
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

    return random.choice(best_moves)"#;

// TEMPORARY
async fn add_system_bots(db_pool: SqlitePool) {
    let new_id = Uuid::from_u128(0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128);
    let _ = sqlx::query!("INSERT INTO bots (id, name, description, source_code, is_active, is_public, is_system, is_valid) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", new_id, "Random bot", "All moves are pure random", RANDOM_BOT,true, true, true, true).execute(&db_pool).await;
    let new_id = Uuid::from_u128(0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d7u128);
    let _ = sqlx::query!("INSERT INTO bots (id, name, description, source_code, is_active, is_public, is_system, is_valid) VALUES (?, ?, ?, ?, ?, ?, ?, ?)", new_id, "Greedy bot", "Takes if he can", GREEDY_BOT,true, true, true, true).execute(&db_pool).await;
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
    let app_state = state::AppState {
        sqlite_pool: db_pool.clone(),
        redis_client: redis_client,
        redis_con: redis_connection,
        oauth_client_id: env::var("OAUTH_CLIENT_ID").unwrap(),
        oauth_client_secret: env::var("OAUTH_CLIENT_SECRET").unwrap(),
        oauth_callback_url: env::var("OAUTH_CALLBACK").unwrap(),
    };

    let cleanup_pool = db_pool.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_hours(2)).await;
            let _ = sqlx::query!("DELETE FROM sessions WHERE expires_at < CURRENT_TIMESTAMP")
                .execute(&cleanup_pool)
                .await;
        }
    });

    let app = app_routes(app_state);
    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("app running at {}", listener.local_addr().unwrap());
    let _ = axum::serve(listener, app).await;
}
