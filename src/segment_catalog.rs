use serde::Serialize;

const SCHEMA_SOURCE: &str = include_str!("config_schema.rs");
const SEGMENT_SPEC_ENUM: &str = "SegmentSpec";
const COLOR_ENUM: &str = "Color";

#[derive(Debug, Serialize)]
pub struct Catalog {
    pub version: &'static str,
    pub segments: Vec<SegmentDoc>,
}

#[derive(Debug, Serialize)]
pub struct SegmentDoc {
    pub name: String,
    pub pitch: String,
    pub fields: Vec<FieldDoc>,
}

#[derive(Debug, Serialize)]
pub struct FieldDoc {
    pub name: String,
    pub hint: String,
    pub kind: FieldKind,
    pub optional: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    Color,
    Text,
    Bool,
    Integer,
    Float,
    List,
    Pairs,
    Enum,
}

pub fn to_json() -> Result<String, String> {
    let catalog = parse(SCHEMA_SOURCE)?;

    serde_json::to_string_pretty(&catalog).map_err(|error| error.to_string())
}

pub fn parse(source: &str) -> Result<Catalog, String> {
    let mut parser = Parser::default();

    for (index, line) in source.lines().enumerate() {
        parser.feed(index + 1, line.trim())?;
    }

    parser.into_catalog()
}

struct RawField {
    name: String,
    hint: Option<String>,
    type_name: String,
    optional: bool,
}

struct RawStruct {
    name: String,
    pitch: Option<String>,
    fields: Vec<RawField>,
}

struct RawEnum {
    name: String,
    variants: Vec<String>,
}

enum Block {
    Struct(RawStruct),
    Enum(RawEnum),
}

#[derive(Default)]
struct Parser {
    pitch: Option<String>,
    open_attribute: Option<String>,
    next_field_optional: bool,
    next_field_hint: Option<String>,
    block: Option<Block>,
    structs: Vec<RawStruct>,
    enums: Vec<RawEnum>,
}

impl Parser {
    fn feed(&mut self, number: usize, line: &str) -> Result<(), String> {
        if let Some(open) = self.open_attribute.as_mut() {
            open.push(' ');
            open.push_str(line);

            if line.ends_with(']') {
                let attribute = self.open_attribute.take().unwrap_or_default();
                self.finish_attribute(&attribute);
            }

            return Ok(());
        }

        if line.starts_with("#[") {
            if line.ends_with(']') {
                self.finish_attribute(line);
            } else {
                self.open_attribute = Some(line.to_string());
            }

            return Ok(());
        }

        if let Some(doc) = line.strip_prefix("///") {
            return self.feed_doc_line(number, doc.trim());
        }

        match self.block.take() {
            Some(Block::Struct(item)) => self.feed_struct_line(number, line, item),
            Some(Block::Enum(item)) => self.feed_enum_line(number, line, item),
            None => self.feed_top_line(line),
        }
    }

    fn feed_doc_line(&mut self, number: usize, doc: &str) -> Result<(), String> {
        match &self.block {
            Some(Block::Struct(_)) => {
                if self.next_field_hint.is_some() {
                    return Err(format!(
                        "line {number}: a field hint must be a single /// line"
                    ));
                }

                self.next_field_hint = Some(doc.to_string());
            }
            Some(Block::Enum(_)) => {
                return Err(format!("line {number}: enum variants carry no /// lines"));
            }
            None => {
                if self.pitch.is_some() {
                    return Err(format!(
                        "line {number}: a segment pitch must be a single /// line"
                    ));
                }

                self.pitch = Some(doc.to_string());
            }
        }

        Ok(())
    }

    fn finish_attribute(&mut self, attribute: &str) {
        if attribute.starts_with("#[serde(") && attribute.contains("default") {
            self.next_field_optional = true;
        }
    }

    fn feed_top_line(&mut self, line: &str) -> Result<(), String> {
        if let Some(name) = item_name(line, "pub struct ") {
            self.block = Some(Block::Struct(RawStruct {
                name,
                pitch: self.pitch.take(),
                fields: Vec::new(),
            }));
        } else if let Some(name) = item_name(line, "pub enum ") {
            self.pitch = None;
            self.block = Some(Block::Enum(RawEnum {
                name,
                variants: Vec::new(),
            }));
        } else if !line.is_empty() {
            self.pitch = None;
        }

        Ok(())
    }

    fn feed_struct_line(
        &mut self,
        number: usize,
        line: &str,
        mut item: RawStruct,
    ) -> Result<(), String> {
        if line == "}" {
            self.structs.push(item);
            self.next_field_optional = false;
            self.next_field_hint = None;
            return Ok(());
        }

        if let Some(field) = line.strip_prefix("pub ") {
            let (name, type_name) = field.split_once(':').ok_or_else(|| {
                format!(
                    "line {number}: cannot read a field out of '{line}' in {}",
                    item.name
                )
            })?;

            item.fields.push(RawField {
                name: name.trim().to_string(),
                hint: self.next_field_hint.take(),
                type_name: type_name.trim().trim_end_matches(',').to_string(),
                optional: std::mem::take(&mut self.next_field_optional),
            });
        } else if !line.is_empty() {
            return Err(format!(
                "line {number}: unexpected '{line}' inside struct {}",
                item.name
            ));
        }

        self.block = Some(Block::Struct(item));
        Ok(())
    }

    fn feed_enum_line(
        &mut self,
        number: usize,
        line: &str,
        mut item: RawEnum,
    ) -> Result<(), String> {
        if line == "}" {
            self.enums.push(item);
            return Ok(());
        }

        if !line.is_empty() {
            let variant: String = line
                .chars()
                .take_while(|character| character.is_alphanumeric() || *character == '_')
                .collect();

            if variant.is_empty() {
                return Err(format!(
                    "line {number}: unexpected '{line}' inside enum {}",
                    item.name
                ));
            }

            item.variants.push(variant);
        }

        self.block = Some(Block::Enum(item));
        Ok(())
    }

    fn into_catalog(self) -> Result<Catalog, String> {
        let spec = self
            .enums
            .iter()
            .find(|item| item.name == SEGMENT_SPEC_ENUM)
            .ok_or_else(|| format!("enum {SEGMENT_SPEC_ENUM} not found"))?;

        for item in &self.structs {
            if item.pitch.is_some() && !spec.variants.contains(&item.name) {
                return Err(format!(
                    "struct {} carries a pitch but is not a {SEGMENT_SPEC_ENUM} variant",
                    item.name
                ));
            }
        }

        let mut segments = Vec::with_capacity(spec.variants.len());

        for name in &spec.variants {
            let item = self
                .structs
                .iter()
                .find(|item| &item.name == name)
                .ok_or_else(|| format!("{SEGMENT_SPEC_ENUM}::{name} has no struct {name}"))?;

            let pitch = item
                .pitch
                .as_deref()
                .filter(|pitch| !pitch.is_empty())
                .ok_or_else(|| format!("segment {name} has no /// pitch above its struct"))?;

            let fields = item
                .fields
                .iter()
                .map(|field| self.describe_field(name, field))
                .collect::<Result<Vec<_>, _>>()?;

            segments.push(SegmentDoc {
                name: name.clone(),
                pitch: pitch.to_string(),
                fields,
            });
        }

        Ok(Catalog {
            version: env!("CARGO_PKG_VERSION"),
            segments,
        })
    }

    fn describe_field(&self, segment: &str, field: &RawField) -> Result<FieldDoc, String> {
        let (kind, variants) = match field.type_name.as_str() {
            COLOR_ENUM => (FieldKind::Color, Vec::new()),
            "String" => (FieldKind::Text, Vec::new()),
            "bool" => (FieldKind::Bool, Vec::new()),
            "u8" | "u16" | "u32" | "u64" | "usize" => (FieldKind::Integer, Vec::new()),
            "f32" | "f64" => (FieldKind::Float, Vec::new()),
            "Vec<String>" => (FieldKind::List, Vec::new()),
            "Vec<(String, String)>" => (FieldKind::Pairs, Vec::new()),
            other => {
                let variants = self
                    .enums
                    .iter()
                    .find(|item| item.name == other && item.name != SEGMENT_SPEC_ENUM)
                    .map(|item| item.variants.clone())
                    .ok_or_else(|| {
                        format!(
                            "segment {segment} field {} has type {other}, which the catalogue cannot describe",
                            field.name
                        )
                    })?;

                (FieldKind::Enum, variants)
            }
        };

        let hint = field
            .hint
            .as_deref()
            .filter(|hint| !hint.is_empty())
            .ok_or_else(|| {
                format!(
                    "segment {segment} field {} has no /// hint above it",
                    field.name
                )
            })?;

        Ok(FieldDoc {
            name: field.name.clone(),
            hint: hint.to_string(),
            kind,
            optional: field.optional,
            variants,
        })
    }
}

fn item_name(line: &str, prefix: &str) -> Option<String> {
    let rest = line.strip_prefix(prefix)?;
    let name: String = rest
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .collect();

    (!name.is_empty() && rest[name.len()..].trim() == "{").then_some(name)
}

#[cfg(test)]
mod tests {
    use super::{parse, to_json, FieldDoc, FieldKind, SCHEMA_SOURCE};

    fn schema(body: &str) -> String {
        format!(
            concat!(
                "pub enum Color {{\n    Named(u8),\n    Gradient,\n}}\n",
                "{}\n",
                "pub enum SegmentSpec {{\n    Foo(Foo),\n}}\n",
                "pub struct RootConfig {{\n    pub segments: Vec<SegmentSpec>,\n}}\n"
            ),
            body
        )
    }

    fn field<'a>(fields: &'a [FieldDoc], name: &str) -> &'a FieldDoc {
        fields
            .iter()
            .find(|field| field.name == name)
            .unwrap_or_else(|| panic!("field {name}"))
    }

    #[test]
    fn describes_every_segment_of_the_real_schema() {
        let catalog = parse(SCHEMA_SOURCE).unwrap();
        let structs = SCHEMA_SOURCE
            .lines()
            .filter(|line| line.starts_with("pub struct "))
            .count();

        assert_eq!(catalog.segments.len(), structs - 1);
        assert_eq!(catalog.version, env!("CARGO_PKG_VERSION"));

        for segment in &catalog.segments {
            assert!(!segment.pitch.is_empty(), "{}", segment.name);
            assert!(!segment.fields.is_empty(), "{}", segment.name);
        }

        let spacer = catalog
            .segments
            .iter()
            .find(|segment| segment.name == "Spacer")
            .unwrap();
        let shape = field(&spacer.fields, "shape");
        assert_eq!(shape.kind, FieldKind::Enum);
        assert_eq!(shape.variants, ["Gap", "LineBreak", "BlankLine"]);

        let rate_limit = catalog
            .segments
            .iter()
            .find(|segment| segment.name == "RateLimit")
            .unwrap();
        let window = field(&rate_limit.fields, "window");
        assert_eq!(window.kind, FieldKind::Enum);
        assert_eq!(window.variants, ["FiveHour", "SevenDay", "Fable"]);
        let midpoint = field(&rate_limit.fields, "gradient_midpoint_percentage");
        assert_eq!(midpoint.kind, FieldKind::Float);
        assert!(midpoint.optional);
        assert_eq!(
            field(&rate_limit.fields, "severity_markers").kind,
            FieldKind::Pairs
        );
        assert!(!field(&rate_limit.fields, "low_color").optional);

        let command_output = catalog
            .segments
            .iter()
            .find(|segment| segment.name == "CommandOutput")
            .unwrap();
        assert_eq!(field(&command_output.fields, "args").kind, FieldKind::List);
        assert_eq!(
            field(&command_output.fields, "ttl_seconds").kind,
            FieldKind::Integer
        );
    }

    #[test]
    fn prints_json_with_the_crate_version() {
        let json = to_json().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(parsed["segments"][0]["fields"][0]["kind"], "color");
    }

    #[test]
    fn reads_the_pitch_and_the_fields_of_a_segment() {
        let source = schema(concat!(
            "/// What foo shows.\n",
            "#[derive(Deserialize)]\n",
            "pub struct Foo {\n",
            "    /// Colour of foo.\n",
            "    pub color: Color,\n",
            "    /// Text before foo.\n",
            "    #[serde(default)]\n",
            "    pub prefix: String,\n",
            "    /// Whether foo is on.\n",
            "    pub enabled: bool,\n",
            "    /// How many seconds foo waits.\n",
            "    pub seconds: u64,\n",
            "    /// Arguments foo is started with.\n",
            "    pub args: Vec<String>,\n",
            "    /// Pairs foo replaces.\n",
            "    pub pairs: Vec<(String, String)>,\n",
            "}\n"
        ));

        let catalog = parse(&source).unwrap();
        let segment = &catalog.segments[0];

        assert_eq!(segment.name, "Foo");
        assert_eq!(segment.pitch, "What foo shows.");
        assert_eq!(field(&segment.fields, "color").kind, FieldKind::Color);
        assert_eq!(field(&segment.fields, "color").hint, "Colour of foo.");
        assert_eq!(field(&segment.fields, "prefix").hint, "Text before foo.");
        assert!(!field(&segment.fields, "color").optional);
        assert_eq!(field(&segment.fields, "prefix").kind, FieldKind::Text);
        assert!(field(&segment.fields, "prefix").optional);
        assert!(!field(&segment.fields, "enabled").optional);
        assert_eq!(field(&segment.fields, "seconds").kind, FieldKind::Integer);
        assert_eq!(field(&segment.fields, "args").kind, FieldKind::List);
        assert_eq!(field(&segment.fields, "pairs").kind, FieldKind::Pairs);
    }

    #[test]
    fn treats_a_multi_line_serde_default_as_optional() {
        let source = schema(concat!(
            "/// Foo.\n",
            "#[derive(Deserialize)]\n",
            "pub struct Foo {\n",
            "    /// Ratio of foo.\n",
            "    #[serde(\n",
            "        default = \"default_ratio\",\n",
            "        deserialize_with = \"deserialize_ratio\"\n",
            "    )]\n",
            "    pub ratio: f64,\n",
            "    /// Colour of foo.\n",
            "    pub color: Color,\n",
            "}\n"
        ));

        let segment = &parse(&source).unwrap().segments[0];

        assert!(field(&segment.fields, "ratio").optional);
        assert!(!field(&segment.fields, "color").optional);
    }

    #[test]
    fn rejects_a_field_without_a_hint() {
        let source = schema("/// Foo.\n#[derive(Deserialize)]\npub struct Foo {\n    pub color: Color,\n}\n");

        let error = parse(&source).unwrap_err();

        assert!(error.contains("field color has no /// hint"), "{error}");
    }

    #[test]
    fn rejects_a_segment_without_a_pitch() {
        let source = schema("#[derive(Deserialize)]\npub struct Foo {\n    pub color: Color,\n}\n");

        let error = parse(&source).unwrap_err();

        assert!(error.contains("Foo has no /// pitch"), "{error}");
    }

    #[test]
    fn rejects_a_pitch_longer_than_one_line() {
        let source = schema(concat!(
            "/// Foo.\n",
            "/// More about foo.\n",
            "pub struct Foo {\n",
            "    pub color: Color,\n",
            "}\n"
        ));

        let error = parse(&source).unwrap_err();

        assert!(error.contains("single /// line"), "{error}");
    }

    #[test]
    fn rejects_a_field_type_it_cannot_describe() {
        let source = schema("/// Foo.\npub struct Foo {\n    pub delay: Duration,\n}\n");

        let error = parse(&source).unwrap_err();

        assert!(error.contains("field delay has type Duration"), "{error}");
    }

    #[test]
    fn rejects_a_pitched_struct_that_is_not_a_segment() {
        let source = schema(concat!(
            "/// Foo.\npub struct Foo {\n    pub color: Color,\n}\n",
            "/// Bar.\npub struct Bar {\n    pub color: Color,\n}\n"
        ));

        let error = parse(&source).unwrap_err();

        assert!(error.contains("struct Bar carries a pitch"), "{error}");
    }
}
