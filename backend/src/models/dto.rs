use serde::Deserialize;

#[derive(Deserialize)]
pub struct OAuthRequest {
    pub code: String,
    pub state: String,
}

#[derive(Deserialize)]
pub struct OAuthResponse {
    pub access_token: String,
    pub scope: String,
    pub token_type: String
}

#[derive(Deserialize)]
pub struct GithubResponse {
    pub login: String,
    pub id: u32,
    pub avatar_url: String,
    pub name: String,
    pub email: Option<String>
}