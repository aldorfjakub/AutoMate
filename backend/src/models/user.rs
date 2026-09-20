use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct BasicUserInfo {
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}
