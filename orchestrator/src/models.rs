
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum ValidationStatus {
    Pending,
    Running,
    Validated,
    Failed { reason: String },
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
        white_bot_id: String,
        black_bot_id: String,
    },
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
}

pub async fn get_bot(pool: &SqlitePool, id: Uuid) -> Result<Option<BotInfo>, sqlx::Error> {
    sqlx::query_as!(BotInfo, r#"SELECT id as "id: uuid::Uuid", name, description, is_active, is_public, source_code, is_valid FROM bots WHERE id = ?"#, id)
        .fetch_optional(pool)
        .await
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "status")]
pub enum MatchStatus {
    Pending,
    Running,
    Finished { winner: String },
    Failed { reason: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum MatchEvent {
    Move {
        san: String,
        fen: String,
        move_number: u32,
    },
    Finished {
        winner: String,
        reason: String,
    },
    Failed {
        reason: String,
    },
}
