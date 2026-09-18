pub const PROVISIONAL_GAMES: i64 = 30;
pub const K_PROVISIONAL: f64 = 40.0;
pub const K_ESTABLISHED: f64 = 20.0;

pub fn k_factor(total_matches: i64) -> f64 {
    if total_matches < PROVISIONAL_GAMES {
        K_PROVISIONAL
    } else {
        K_ESTABLISHED
    }
}

pub fn expected_score(rating: f64, opponent_rating: f64) -> f64 {
    1.0 / (1.0 + 10.0_f64.powf((opponent_rating - rating) / 400.0))
}

pub fn rating_delta(rating: f64, opponent_rating: f64, score: f64, k: f64) -> f64 {
    k * (score - expected_score(rating, opponent_rating))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_rating_of_equal_players_is_half() {
        assert!((expected_score(1000.0, 1000.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn higher_rated_player_is_favored() {
        let fav = expected_score(1200.0, 1000.0);
        let dog = expected_score(1000.0, 1200.0);
        assert!(fav > 0.5);
        assert!((fav + dog - 1.0).abs() < 1e-9);
    }

    #[test]
    fn favorite_win_gains_less_than_upset_win() {
        let k = 40.0;
        let favorite_win = rating_delta(1200.0, 1000.0, 1.0, k);
        let upset_win = rating_delta(1000.0, 1200.0, 1.0, k);
        assert!(upset_win > favorite_win);
        assert!(favorite_win > 0.0);
    }

    #[test]
    fn loss_losses_same_magnitude_as_win_gain() {
        let k = 40.0;
        assert!((rating_delta(1000.0, 1000.0, 1.0, k)
            + rating_delta(1000.0, 1000.0, 0.0, k))
        .abs()
            < 1e-9);
    }

    #[test]
    fn draw_moves_ratings_toward_average() {
        assert!(rating_delta(1000.0, 1200.0, 0.5, 40.0) > 0.0);
        assert!(rating_delta(1200.0, 1000.0, 0.5, 40.0) < 0.0);
    }

    #[test]
    fn k_factor_drops_after_thirty_games() {
        assert_eq!(k_factor(0), 40.0);
        assert_eq!(k_factor(29), 40.0);
        assert_eq!(k_factor(30), 20.0);
    }
}