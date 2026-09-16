use serde_json::Value;

pub use crate::config_schema::TurnCost;
use crate::dollar_amount_format::format_dollars;
use crate::segments::{GitCache, Segment};
use crate::session_token_tallies::load_session_tallies;
use crate::statusline_input::session_key;
use crate::turn_cost_baseline::{self, TurnCostBaseline};

impl Segment for TurnCost {
    fn render(&self, json: &Value, _git: &mut GitCache) -> Option<String> {
        let total = json.pointer("/cost/total_cost_usd")?.as_f64()?;
        let session = session_key(json)?;
        let turn_started_at = load_session_tallies(json)?.turn_started_at();
        let known = turn_cost_baseline::load(&session);
        let current = TurnCostBaseline::advance(known.clone(), turn_started_at, total);

        if known.as_ref() != Some(&current) {
            turn_cost_baseline::store(&session, &current);
        }

        let text = self.format(current.turn_cost())?;

        Some(self.color.paint(&text))
    }
}

impl TurnCost {
    fn format(&self, dollars: f64) -> Option<String> {
        Some(format!("{}{}", self.prefix, format_dollars(dollars)?))
    }
}

#[cfg(test)]
mod tests {
    use super::TurnCost;
    use crate::config_schema::Color;

    fn cost() -> TurnCost {
        TurnCost {
            color: Color::Named(90),
            prefix: "turn ".to_string(),
        }
    }

    #[test]
    fn shows_the_turn_cost_after_its_prefix() {
        assert_eq!(cost().format(0.35).as_deref(), Some("turn $0.35"));
    }

    #[test]
    fn hides_until_the_turn_has_cost_something() {
        assert_eq!(cost().format(0.0), None);
    }
}
