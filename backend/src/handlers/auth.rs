use std::{str::FromStr, sync::Arc};

use crate::models::dto::*;
use crate::{
    error::{AppError, AppResult},
    state::AppState,
};
use axum::{
    Json, Router,
    extract::State,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};
use uuid::Uuid;

const OAUTH_PROVIDER: &str = "github";
const OAUTH_SCOPE: &str = "read%3Auser";
const OAUTH_URL: &str = "https://github.com/login/oauth/authorize";
const OAUTH_USER_URL: &str = "https://api.github.com/user";
const OAUTH_ROOT_URL: &str = "https://github.com/login/oauth/access_token";

async fn login_oauth(jar: CookieJar, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let oauth_state = Uuid::new_v4().to_string();

    let url = format!(
        "{}?client_id={}&redirect_uri={}&scope={}&state={}",
        OAUTH_URL, state.oauth_client_id, state.oauth_callback_url, OAUTH_SCOPE, &oauth_state
    );

    let cookie = Cookie::build(("oauth_state", oauth_state))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .build();

    (jar.add(cookie), Redirect::to(&url))
}

async fn create_user(github_response: &GithubResponse, state: Arc<AppState>) -> AppResult<Vec<u8>> {
    // TODO make atomic, sqlitepool instead of state, move into services
    let user_id = Uuid::now_v7();
    let user_id_bytes = user_id.clone().as_bytes().to_vec();
    let email = &github_response.email;
    let display_name = &github_response.name;
    let avatar_url = &github_response.avatar_url;

    let _ = sqlx::query!(
        "INSERT INTO users (id, email, display_name, avatar_url) VALUES (?, ?, ?, ?)",
        user_id_bytes,
        email,
        display_name,
        avatar_url
    )
    .execute(&state.sqlite_pool)
    .await?;

    let _ = sqlx::query!(
        "INSERT INTO oauth_accounts (user_id, provider, provider_user_id) VALUES (?, ?, ?)",
        user_id_bytes,
        OAUTH_PROVIDER,
        github_response.id
    )
    .execute(&state.sqlite_pool)
    .await?;

    Ok(user_id_bytes)
}

async fn oauth_callback(
    jar: CookieJar,
    State(state): State<Arc<AppState>>,
    Json(body): Json<OAuthRequest>,
) -> AppResult<(CookieJar, Redirect)> {
    let cookie = jar.get("oauth_state").ok_or(AppError::Unauthorized)?;
    if cookie.value() != body.state {
        return Err(AppError::Unauthorized);
    }
    let client = reqwest::Client::new();

    let url = format!(
        "{}?client_id={}&client_secret={}&code={}",
        OAUTH_ROOT_URL, state.oauth_client_id, state.oauth_client_secret, body.code
    );

    //Create find / create user
    //Create new session and return id in the cookie
    let res = client
        .post(url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::External(e.to_string()))?;
    let content = res
        .json::<OAuthResponse>()
        .await
        .map_err(|e| AppError::External(e.to_string()))?;
    let token = content.access_token;

    let res2 = client
        .get(OAUTH_USER_URL)
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "AutoMate")
        .send()
        .await
        .map_err(|e| AppError::External(e.to_string()))?;

    //println!("Response text is: {}", res2.unwrap().text().await.unwrap());
    let github_response = res2
        .json::<GithubResponse>()
        .await
        .map_err(|e| AppError::External(e.to_string()))?;
    println!("Github user: {}", &github_response.login);

    //Here you would check if the user exists in the database, if not create it, then create a session and return the session id in a cookie
    let query = sqlx::query!(
        "SELECT user_id FROM oauth_accounts WHERE provider = ? AND provider_user_id = ? ",
        OAUTH_PROVIDER,
        github_response.id
    )
    .fetch_one(&state.sqlite_pool)
    .await;

    let user_id = match query {
        Ok(record) => record.user_id,
        Err(sqlx::Error::RowNotFound) => create_user(&github_response, state.clone()).await?,
        Err(e) => {
            return Err(AppError::Database(e));
        }
    };

    let session_id = Uuid::now_v7();
    let session_id_bytes = session_id.as_bytes().to_vec();
    let _ = sqlx::query!(
        "INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, DATETIME('now', '+7 day'))",
        session_id_bytes,
        user_id
    )
    .execute(&state.sqlite_pool)
    .await?;
    let cookie = Cookie::build(("session_id", session_id.to_string()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .build();

    Ok((jar.add(cookie), Redirect::to("/")))
}

async fn logout(
    jar: CookieJar,
    State(state): State<Arc<AppState>>,
) -> AppResult<(CookieJar, Redirect)> {
    let session_id = jar.get("session_id").ok_or(AppError::Unauthorized)?.value();
    let id_bytes = Uuid::from_str(session_id)
        .map_err(|_| AppError::BadRequest("invalid session_id"))?
        .as_bytes()
        .to_vec();

    let _ = sqlx::query!("DELETE FROM sessions WHERE id = ?", id_bytes)
        .execute(&state.sqlite_pool)
        .await?;
    let removal = Cookie::build("session_id")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax);
    let jar = jar.remove(removal);
    Ok((jar, Redirect::to("/")))
}

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/oauth-login", get(login_oauth))
        .route("/callback", post(oauth_callback))
        .route("/logout", post(logout))
}
