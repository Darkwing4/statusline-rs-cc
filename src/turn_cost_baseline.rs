use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::private_file;
use crate::statusline_cache_dir::cache_dir;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub(crate) struct TurnCostBaseline {
    turn_started_at: Option<i64>,
    baseline: f64,
    last_total: f64,
}

impl TurnCostBaseline {
    pub(crate) fn advance(
        known: Option<TurnCostBaseline>,
        turn_started_at: Option<i64>,
        total: f64,
    ) -> TurnCostBaseline {
        let baseline = match known {
            None => total,
            Some(known) if total < known.last_total => 0.0,
            Some(known) if known.turn_started_at != turn_started_at => known.last_total,
            Some(known) => known.baseline,
        };

        TurnCostBaseline {
            turn_started_at,
            baseline,
            last_total: total,
        }
    }

    pub(crate) fn turn_cost(&self) -> f64 {
        self.last_total - self.baseline
    }
}

pub(crate) fn load(session_key: &str) -> Option<TurnCostBaseline> {
    let body = fs::read_to_string(baseline_path(session_key)?).ok()?;

    serde_json::from_str(&body).ok()
}

pub(crate) fn store(session_key: &str, baseline: &TurnCostBaseline) {
    let Some(path) = baseline_path(session_key) else {
        return;
    };

    let Some(parent) = path.parent() else {
        return;
    };

    if fs::create_dir_all(parent).is_err() {
        return;
    }

    let Ok(body) = serde_json::to_string(baseline) else {
        return;
    };

    let _ = private_file::write(&path, body);
}

fn baseline_path(session_key: &str) -> Option<PathBuf> {
    Some(cache_dir()?.join(format!("turn-cost-{}.json", file_key(session_key))))
}

fn file_key(session_key: &str) -> String {
    session_key
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::TurnCostBaseline;

    #[test]
    fn attributes_nothing_to_a_turn_it_did_not_watch_from_the_start() {
        let first = TurnCostBaseline::advance(None, Some(100), 4.2);

        assert_eq!(first.turn_cost(), 0.0);
    }

    #[test]
    fn grows_with_the_total_inside_one_turn() {
        let start = TurnCostBaseline::advance(None, Some(100), 4.2);
        let later = TurnCostBaseline::advance(Some(start), Some(100), 4.5);

        assert!((later.turn_cost() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn restarts_from_the_previous_turn_total_when_a_new_prompt_arrives() {
        let start = TurnCostBaseline::advance(None, Some(100), 4.0);
        let end_of_turn = TurnCostBaseline::advance(Some(start), Some(100), 4.5);
        let new_turn = TurnCostBaseline::advance(Some(end_of_turn), Some(200), 4.5);
        let answered = TurnCostBaseline::advance(Some(new_turn.clone()), Some(200), 4.7);

        assert_eq!(new_turn.turn_cost(), 0.0);
        assert!((answered.turn_cost() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn keeps_the_missed_spend_in_the_turn_when_the_first_render_comes_late() {
        let end_of_turn = TurnCostBaseline::advance(None, Some(100), 4.5);
        let late = TurnCostBaseline::advance(Some(end_of_turn), Some(200), 4.8);

        assert!((late.turn_cost() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn counts_from_zero_again_after_the_session_total_resets() {
        let before = TurnCostBaseline::advance(None, Some(100), 4.5);
        let same_turn = TurnCostBaseline::advance(Some(before.clone()), Some(100), 0.1);
        let next_turn = TurnCostBaseline::advance(Some(before), Some(200), 0.1);

        assert!((same_turn.turn_cost() - 0.1).abs() < 1e-9);
        assert!((next_turn.turn_cost() - 0.1).abs() < 1e-9);
    }
}
