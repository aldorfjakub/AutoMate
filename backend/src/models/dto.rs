use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
}

#[derive(Serialize, Deserialize)]
pub struct BotSummary {
    pub id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub is_public: bool,
    pub is_valid: bool,
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
    Finished {winner: String},
    Failed {reason: String}
}

#[derive(Serialize, Deserialize)]
pub struct MatchRequest{
    pub player_bot_id: Uuid,
    pub opponent_bot_id: Uuid
}