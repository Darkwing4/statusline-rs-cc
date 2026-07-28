use std::cell::Cell;
use std::fmt;

use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

use crate::transcript_tail_reader::JsonlRecord;

pub(crate) fn has_type(record: &mut dyn JsonlRecord, expected: &str) -> bool {
    let matched = Cell::new(None);
    let seed = RecordTypeSeed {
        expected,
        matched: &matched,
    };
    let mut deserializer = serde_json::Deserializer::from_reader(record);
    let _ = seed.deserialize(&mut deserializer);
    matched.get().unwrap_or(false)
}

pub(crate) fn has_tool_result(record: &mut dyn JsonlRecord) -> bool {
    let found = Cell::new(false);
    let seed = ToolResultRecordSeed { found: &found };
    let mut deserializer = serde_json::Deserializer::from_reader(record);
    let _ = seed.deserialize(&mut deserializer);
    found.get()
}

struct RecordTypeSeed<'a> {
    expected: &'a str,
    matched: &'a Cell<Option<bool>>,
}

impl<'de> DeserializeSeed<'de> for RecordTypeSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(RecordTypeVisitor {
            expected: self.expected,
            matched: self.matched,
        })
    }
}

struct RecordTypeVisitor<'a> {
    expected: &'a str,
    matched: &'a Cell<Option<bool>>,
}

impl<'de> Visitor<'de> for RecordTypeVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a transcript record")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<String>()? {
            if key == "type" {
                let value = map.next_value::<Value>()?;
                self.matched
                    .set(Some(value.as_str() == Some(self.expected)));
                return Err(de::Error::custom("transcript record type found"));
            }

            map.next_value::<IgnoredAny>()?;
        }

        self.matched.set(Some(false));
        Ok(())
    }
}

struct ToolResultRecordSeed<'a> {
    found: &'a Cell<bool>,
}

impl<'de> DeserializeSeed<'de> for ToolResultRecordSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ToolResultRecordVisitor { found: self.found })
    }
}

struct ToolResultRecordVisitor<'a> {
    found: &'a Cell<bool>,
}

impl<'de> Visitor<'de> for ToolResultRecordVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a transcript record")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<String>()? {
            if key == "message" {
                map.next_value_seed(MessageSeed { found: self.found })?;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }

        Ok(())
    }
}

struct MessageSeed<'a> {
    found: &'a Cell<bool>,
}

impl<'de> DeserializeSeed<'de> for MessageSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(MessageVisitor { found: self.found })
    }
}

struct MessageVisitor<'a> {
    found: &'a Cell<bool>,
}

impl<'de> Visitor<'de> for MessageVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a transcript message")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<String>()? {
            if key == "content" {
                map.next_value_seed(ContentSeed { found: self.found })?;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }

        Ok(())
    }
}

struct ContentSeed<'a> {
    found: &'a Cell<bool>,
}

impl<'de> DeserializeSeed<'de> for ContentSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ContentVisitor { found: self.found })
    }
}

struct ContentVisitor<'a> {
    found: &'a Cell<bool>,
}

impl<'de> Visitor<'de> for ContentVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("transcript message content")
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence
            .next_element_seed(ContentBlockSeed { found: self.found })?
            .is_some()
        {}

        Ok(())
    }
}

struct ContentBlockSeed<'a> {
    found: &'a Cell<bool>,
}

impl<'de> DeserializeSeed<'de> for ContentBlockSeed<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(ContentBlockVisitor { found: self.found })
    }
}

struct ContentBlockVisitor<'a> {
    found: &'a Cell<bool>,
}

impl<'de> Visitor<'de> for ContentBlockVisitor<'_> {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a transcript content block")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        while let Some(key) = map.next_key::<String>()? {
            if key == "type" {
                let value = map.next_value::<Value>()?;
                if value.as_str() == Some("tool_result") {
                    self.found.set(true);
                    return Err(de::Error::custom("tool result found"));
                }
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }

        Ok(())
    }
}
