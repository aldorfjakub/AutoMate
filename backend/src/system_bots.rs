use sqlx::SqlitePool;
use uuid::Uuid;

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

const SYSTEM_BOTS: [(Uuid, &str, &str, &str); 2] = [
    (
        Uuid::from_u128(0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d8u128),
        "Random bot",
        "All moves are pure random",
        RANDOM_BOT,
    ),
    (
        Uuid::from_u128(0xa1a2a3a4b1b2c1c2d1d2d3d4d5d6d7d6u128),
        "Greedy bot",
        "Takes if he can",
        GREEDY_BOT,
    ),
];

pub async fn add_system_bots(db_pool: &SqlitePool) {
    for (id, name, description, source_code) in SYSTEM_BOTS {
        let _ = sqlx::query!(
            r#"INSERT INTO bots (id, name, description, source_code, is_active, is_public, is_system, is_valid)
               VALUES (?, ?, ?, ?, 1, 1, 1, 1)
               ON CONFLICT(id) DO UPDATE SET
                   name = excluded.name,
                   description = excluded.description,
                   source_code = excluded.source_code,
                   is_active = 1,
                   is_public = 1,
                   is_valid = 1"#,
            id, name, description, source_code,
        )
        .execute(db_pool)
        .await;
    }
}
