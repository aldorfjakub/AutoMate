pub mod auth;
pub mod bots;
pub mod user;
pub mod jobs;
pub mod play;
pub mod matches;
pub mod leaderboard;

pub use auth::auth_routes;
pub use bots::bots_routes;
pub use user::user_routes;
pub use jobs::job_routes;
pub use play::play_routes;
pub use matches::matches_routes;
pub use leaderboard::leaderboard_routes;
