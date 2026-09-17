use serde_json::Value;

pub use crate::config_schema::SessionCost;
use crate::segments::{GitCache, Segment};
use crate::session_token_tallies::load_session_tallies;
use crate::statusline_input::session_key;
use crate::turn_cost_baseline::{self, TurnCostBaseline};

impl Segment for SessionCost {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let total = json.pointer("/cost/total_cost_usd")?.as_f64()?;
        let turn = turn_cost(json, total).unwrap_or(0.0);
        let text = self.format(total, turn)?;

        Some(self.color.paint(&text))
    }
}

impl SessionCost {
    fn format(&self, total: f64, turn: f64) -> Option<String> {
        if !total.is_finite() || total <= 0.0 {
            return None;
        }

        let mut text = format!("{}${:.0}", self.prefix, total);
        let turn_dimes = (turn * 10.0).round();

        if turn_dimes.is_finite() && turn_dimes >= 1.0 {
            text.push_str(&format!("(+{:.1})", turn_dimes / 10.0));
        }

        Some(text)
    }
}

fn turn_cost(json: &Value, total: f64) -> Option<f64> {
    let session = session_key(json)?;
    let turn_started_at = load_session_tallies(json)?.turn_started_at();
    let known = turn_cost_baseline::load(&session);
    let current = TurnCostBaseline::advance(known.clone(), turn_started_at, total);

    if known.as_ref() != Some(&current) {
        turn_cost_baseline::store(&session, &current);
    }

    Some(current.turn_cost())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::SessionCost;
    use crate::config_schema::Color;
    use crate::segments::{GitCache, Segment};

    fn cost() -> SessionCost {
        SessionCost {
            color: Color::Named(90),
            prefix: String::new(),
        }
    }

    #[test]
    fn shows_the_session_cost_in_whole_dollars() {
        let json = json!({"cost": {"total_cost_usd": 4.2}});
        let mut git = GitCache::new(String::new());

        assert_eq!(
            cost().render(&json, &mut git),
            Some("\x1b[90m$4\x1b[0m".to_string())
        );
    }

    #[test]
    fn adds_the_last_prompt_in_brackets() {
        assert_eq!(cost().format(30.98, 1.0).as_deref(), Some("$31(+1.0)"));
        assert_eq!(cost().format(30.98, 0.3).as_deref(), Some("$31(+0.3)"));
        assert_eq!(cost().format(30.98, 12.64).as_deref(), Some("$31(+12.6)"));
    }

    #[test]
    fn leaves_out_a_last_prompt_cheaper_than_ten_cents() {
        assert_eq!(cost().format(30.98, 0.04).as_deref(), Some("$31"));
        assert_eq!(cost().format(30.98, 0.0).as_deref(), Some("$31"));
        assert_eq!(cost().format(30.98, f64::NAN).as_deref(), Some("$31"));
    }

    #[test]
    fn hides_until_the_first_response_is_priced() {
        let mut git = GitCache::new(String::new());

        assert_eq!(cost().render(&json!({}), &mut git), None);
        assert_eq!(
            cost().render(&json!({"cost": {"total_cost_usd": 0}}), &mut git),
            None
        );
        assert_eq!(cost().format(f64::NAN, 1.0), None);
    }
}
