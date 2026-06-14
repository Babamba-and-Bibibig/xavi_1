//! Domain model for development cycle identity and human-readable aliases.

use std::fmt::Write as _;

/// Stable canonical identifier for a development cycle.
pub type DevelopmentCycleId = String;

/// Human-readable alias such as `feature-001`.
pub type DevelopmentCycleAliasValue = String;

/// User-facing category portion of a cycle alias.
pub type DevelopmentCycleCategory = String;

/// Normalized category key used for sequence allocation and collision checks.
pub type DevelopmentCycleCategoryKey = String;

/// Persisted alias reservation for one canonical cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentCycleAlias {
    /// Canonical cycle id. Existing report and trace paths keep using this value.
    pub cycle_id: DevelopmentCycleId,
    /// Reserved human-readable alias.
    pub cycle_alias: DevelopmentCycleAliasValue,
    /// Display category exactly as accepted from the caller.
    pub cycle_category: DevelopmentCycleCategory,
    /// Normalized category key used for max-sequence lookup.
    pub cycle_category_key: DevelopmentCycleCategoryKey,
    /// Numeric sequence, starting from 1 and displayed with at least three digits.
    pub cycle_sequence: u64,
    /// Optional short human title for report/index display.
    pub cycle_title: Option<String>,
    /// Timestamp string supplied by the caller.
    pub created_at: String,
}

/// Validated parts extracted from a full alias or allocated from a category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentCycleAliasParts {
    /// Complete alias value.
    pub cycle_alias: DevelopmentCycleAliasValue,
    /// Display category exactly as accepted from the caller.
    pub cycle_category: DevelopmentCycleCategory,
    /// Normalized category key.
    pub cycle_category_key: DevelopmentCycleCategoryKey,
    /// Numeric sequence.
    pub cycle_sequence: u64,
}

/// Caller request for reserving an alias.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DevelopmentCycleAliasRequest {
    /// Reserve this exact full alias; collisions fail closed.
    FullAlias(String),
    /// Allocate `max(sequence)+1` inside this category.
    Category(String),
}

/// New alias reservation request before persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevelopmentCycleAliasReservation {
    /// Canonical cycle id receiving the alias.
    pub cycle_id: DevelopmentCycleId,
    /// Caller alias request.
    pub request: DevelopmentCycleAliasRequest,
    /// Optional short title.
    pub cycle_title: Option<String>,
    /// Timestamp string supplied by the caller.
    pub created_at: String,
}

impl DevelopmentCycleAliasReservation {
    /// Builds a reservation request after validating the canonical cycle id and title.
    ///
    /// # Errors
    ///
    /// Returns an error when the cycle id or title is unsafe.
    pub fn new(
        cycle_id: impl Into<String>,
        request: DevelopmentCycleAliasRequest,
        cycle_title: Option<String>,
        created_at: impl Into<String>,
    ) -> Result<Self, String> {
        let cycle_id = cycle_id.into();
        validate_development_cycle_id(&cycle_id)?;
        let cycle_title = normalize_cycle_title(cycle_title)?;
        Ok(Self { cycle_id, request, cycle_title, created_at: created_at.into() })
    }

    /// Returns validated alias parts for exact full-alias requests.
    ///
    /// # Errors
    ///
    /// Returns an error when this request is category-only or the full alias is invalid.
    pub fn full_alias_parts(&self) -> Result<DevelopmentCycleAliasParts, String> {
        match &self.request {
            DevelopmentCycleAliasRequest::FullAlias(alias) => {
                validate_development_cycle_alias(alias)
            }
            DevelopmentCycleAliasRequest::Category(_) => {
                Err("category-only alias reservation requires storage sequence allocation".into())
            }
        }
    }

    /// Returns validated category data for category-only requests.
    ///
    /// # Errors
    ///
    /// Returns an error when this request is a full alias or the category is invalid.
    pub fn category_parts(&self) -> Result<(String, String), String> {
        match &self.request {
            DevelopmentCycleAliasRequest::Category(category) => {
                validate_development_cycle_category(category)
            }
            DevelopmentCycleAliasRequest::FullAlias(_) => {
                Err("full alias reservation already carries an explicit sequence".into())
            }
        }
    }
}

/// Validates a canonical cycle id used in report paths.
///
/// # Errors
///
/// Returns an error when the id could escape the report root or is empty.
pub fn validate_development_cycle_id(cycle_id: &str) -> Result<(), String> {
    let trimmed = cycle_id.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed.contains("..")
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.chars().any(char::is_control)
    {
        return Err(format!("unsafe development cycle id: {cycle_id}"));
    }
    Ok(())
}

/// Validates a full alias in `category-NNN` form.
///
/// # Errors
///
/// Returns an error when the category or sequence violates the alias contract.
pub fn validate_development_cycle_alias(alias: &str) -> Result<DevelopmentCycleAliasParts, String> {
    if alias.trim() != alias || alias.is_empty() {
        return Err("cycle alias must not be empty or padded with whitespace".into());
    }
    reject_alias_forbidden_text("cycle alias", alias)?;
    let mut parts = alias.split('-');
    let Some(category) = parts.next() else {
        return Err("cycle alias must be category-NNN".into());
    };
    let Some(sequence_text) = parts.next() else {
        return Err("cycle alias must be category-NNN".into());
    };
    if parts.next().is_some() {
        return Err("cycle alias must contain exactly one hyphen separator".into());
    }
    let (cycle_category, cycle_category_key) = validate_development_cycle_category(category)?;
    if sequence_text.len() < 3 || !sequence_text.chars().all(|character| character.is_ascii_digit())
    {
        return Err("cycle alias sequence must be at least three ASCII digits".into());
    }
    let cycle_sequence = sequence_text
        .parse::<u64>()
        .map_err(|error| format!("invalid cycle alias sequence: {error}"))?;
    if cycle_sequence == 0 {
        return Err("cycle alias sequence starts at 001".into());
    }
    Ok(DevelopmentCycleAliasParts {
        cycle_alias: alias.to_owned(),
        cycle_category,
        cycle_category_key,
        cycle_sequence,
    })
}

/// Validates a category used in `category-NNN` aliases.
///
/// # Errors
///
/// Returns an error when the category has path separators, whitespace, control chars, `..`,
/// hyphens, or unsupported characters.
pub fn validate_development_cycle_category(category: &str) -> Result<(String, String), String> {
    if category.trim() != category || category.is_empty() {
        return Err("cycle alias category must not be empty or padded with whitespace".into());
    }
    reject_alias_forbidden_text("cycle alias category", category)?;
    if category.contains('-') {
        return Err("cycle alias category must not contain an additional hyphen".into());
    }
    if !category.chars().all(is_allowed_cycle_alias_category_char) {
        return Err(
            "cycle alias category allows only Korean characters, ASCII letters, digits, or underscore"
                .into(),
        );
    }
    let key = category.chars().flat_map(char::to_lowercase).collect::<String>();
    Ok((category.to_owned(), key))
}

/// Builds a full alias from a validated category and sequence.
#[must_use]
pub fn format_development_cycle_alias(category: &str, sequence: u64) -> String {
    format!("{category}-{sequence:03}")
}

/// Renders the alias index stored next to report artifacts as `aliases.json`.
#[must_use]
pub fn render_development_cycle_alias_index_json(aliases: &[DevelopmentCycleAlias]) -> String {
    let mut output = String::new();
    output.push('{');
    push_json_field(&mut output, "version", "1", true);
    output.push_str(",\"aliases\":[");
    for (index, alias) in aliases.iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push('{');
        push_json_field(&mut output, "cycle_id", &json_string(&alias.cycle_id), true);
        push_json_field(&mut output, "cycle_alias", &json_string(&alias.cycle_alias), false);
        push_json_field(&mut output, "cycle_category", &json_string(&alias.cycle_category), false);
        push_json_field(
            &mut output,
            "cycle_category_key",
            &json_string(&alias.cycle_category_key),
            false,
        );
        push_json_field(&mut output, "cycle_sequence", &alias.cycle_sequence.to_string(), false);
        push_json_field(
            &mut output,
            "cycle_title",
            &json_optional_string(alias.cycle_title.as_deref()),
            false,
        );
        push_json_field(&mut output, "created_at", &json_string(&alias.created_at), false);
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn normalize_cycle_title(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().any(char::is_control) {
        return Err("cycle title must not contain control characters".into());
    }
    Ok(Some(trimmed.to_owned()))
}

fn reject_alias_forbidden_text(label: &str, value: &str) -> Result<(), String> {
    if value.contains('/')
        || value.contains('\\')
        || value.contains("..")
        || value.chars().any(char::is_whitespace)
        || value.chars().any(char::is_control)
    {
        return Err(format!(
            "{label} must not contain slash, backslash, whitespace, control characters, or .."
        ));
    }
    Ok(())
}

fn is_allowed_cycle_alias_category_char(character: char) -> bool {
    character == '_'
        || character.is_ascii_alphanumeric()
        || ('\u{ac00}'..='\u{d7a3}').contains(&character)
        || ('\u{1100}'..='\u{11ff}').contains(&character)
        || ('\u{3130}'..='\u{318f}').contains(&character)
        || ('\u{a960}'..='\u{a97f}').contains(&character)
        || ('\u{d7b0}'..='\u{d7ff}').contains(&character)
}

fn push_json_field(output: &mut String, key: &str, value: &str, first: bool) {
    if !first {
        output.push(',');
    }
    let _ = write!(output, "{}:{value}", json_string(key));
}

fn json_optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".to_owned(), json_string)
}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{08}' => escaped.push_str("\\b"),
            '\u{0c}' => escaped.push_str("\\f"),
            control if control.is_control() => {
                let _ = write!(escaped, "\\u{:04x}", u32::from(control));
            }
            other => escaped.push(other),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_full_alias_and_normalizes_category_key() {
        let alias = validate_development_cycle_alias("Feature_한글-001").unwrap();

        assert_eq!(alias.cycle_alias, "Feature_한글-001");
        assert_eq!(alias.cycle_category, "Feature_한글");
        assert_eq!(alias.cycle_category_key, "feature_한글");
        assert_eq!(alias.cycle_sequence, 1);
    }

    #[test]
    fn accepts_user_facing_alias_examples() {
        for value in ["A-001", "기초-001", "기능A-001"] {
            assert!(
                validate_development_cycle_alias(value).is_ok(),
                "alias should be accepted: {value:?}"
            );
        }
    }

    #[test]
    fn rejects_unsafe_or_ambiguous_aliases() {
        for value in [
            "feature-000",
            "feature-01",
            "feature-name-001",
            " feature-001",
            "feature-001 ",
            "feature 001",
            "feature/001",
            "feature\\001",
            "feature..name-001",
            "feature-\n001",
        ] {
            assert!(
                validate_development_cycle_alias(value).is_err(),
                "alias should be rejected: {value:?}"
            );
        }
    }

    #[test]
    fn formats_category_sequence_with_minimum_three_digits() {
        assert_eq!(format_development_cycle_alias("feature", 1), "feature-001");
        assert_eq!(format_development_cycle_alias("feature", 1000), "feature-1000");
    }

    #[test]
    fn renders_alias_index_json_with_required_fields() {
        let json = render_development_cycle_alias_index_json(&[DevelopmentCycleAlias {
            cycle_id: "cycle-example".to_owned(),
            cycle_alias: "작업_분류-001".to_owned(),
            cycle_category: "작업_분류".to_owned(),
            cycle_category_key: "작업_분류".to_owned(),
            cycle_sequence: 1,
            cycle_title: Some("별칭 저장".to_owned()),
            created_at: "unix:1".to_owned(),
        }]);

        assert!(json.contains("\"aliases\""));
        assert!(json.contains("\"cycle_alias\":\"작업_분류-001\""));
        assert!(json.contains("\"cycle_sequence\":1"));
        assert!(json.contains("\"cycle_title\":\"별칭 저장\""));
    }
}
