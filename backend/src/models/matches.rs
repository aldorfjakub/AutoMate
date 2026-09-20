use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct Match {
    pub id: Option<Uuid>,
    pub white_bot_id: Uuid,
    pub black_bot_id: Uuid,
    pub match_status: String,
    pub is_ranked: bool,
    pub winner_color: Option<String>,
    pub win_reason: Option<String>,
    pub pgn: Option<String>,
    pub white_elo_change: Option<i64>,
    pub black_elo_change: Option<i64>,
    pub error_message: Option<String>,
    pub created_at: Option<NaiveDateTime>,
    pub completed_at: Option<NaiveDateTime>,
}
