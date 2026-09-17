use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::matches::Match;

#[derive(Deserialize)]
pub struct OAuthRequest {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
pub struct OAuthResponse {
    pub access_token: String,
    pub scope: String,
    pub token_type: String,
}

#[derive(Deserialize)]
pub struct GithubResponse {
    pub login: String,
    pub id: u32,
    pub avatar_url: String,
    pub name: String,
    pub email: Option<String>,
}
#[derive(Deserialize)]
pub struct NewBotRequest {
    pub name: String,
    pub description: Option<String>,
    pub source_code: String,
    pub is_active: bool,
    pub is_public: bool,
}

#[derive(Serialize, Deserialize)]
pub struct BotInfo {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub source_code: Option<String>,
    pub is_active: bool,
    pub is_public: bool,
    pub is_valid: bool,
    pub rating: f64,
    pub total_matches: i64,
}

#[derive(Serialize, Deserialize)]
pub struct BotSummary {
    pub id: Option<Uuid>,
    pub owner_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub is_public: bool,
    pub is_valid: bool,
    pub rating: f64,
    pub total_matches: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Job {
    Validate {
        bot_id: String,
        job_id: String,
    },
    Match {
        match_id: String,
        is_ranked: bool,
        white_bot_id: String,
        black_bot_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ValidationStatus {
    Pending,
    Running,
    Validated,
    Failed { reason: String },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "status")]
pub enum MatchStatus {
    Pending,
    Running,
    Finished { winner: String },
    Failed { reason: String },
}

#[derive(Serialize, Deserialize)]
pub struct MatchRequest {
    pub player_bot_id: Uuid,
    pub opponent_bot_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct RankedMatch {
    #[serde(flatten)]
    pub m: Match,
    pub white_name: String,
    pub black_name: String,
    pub white_rating: f64,
    pub black_rating: f64,
}

#[derive(Serialize, Deserialize)]
pub struct MatchWithBots {
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
    pub white_name: String,
    pub black_name: String,
    pub white_rating: f64,
    pub black_rating: f64,
}
