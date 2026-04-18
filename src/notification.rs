use std::{collections::HashSet, fmt};

use thiserror::Error;

#[derive(Debug, Clone)]
pub struct BodyFieldExclusions {
    fields: HashSet<BodyField>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyField {
    Event,
    Repository,
    Action,
    PriorStatus,
    Ref,
    Commit,
    Sender,
    Url,
    Delivery,
}

#[derive(Debug, Error)]
pub enum BodyFieldError {
    #[error("invalid notification body exclude fields: {invalid}. valid fields: {valid}")]
    Invalid { invalid: String, valid: String },
}

impl BodyFieldExclusions {
    pub fn new(fields: Vec<String>) -> Result<Self, BodyFieldError> {
        let mut invalid = Vec::new();
        let mut parsed = HashSet::new();

        for field in fields {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            match field.parse() {
                Ok(field) => {
                    parsed.insert(field);
                }
                Err(()) => invalid.push(field.to_owned()),
            }
        }

        if !invalid.is_empty() {
            return Err(BodyFieldError::Invalid {
                invalid: invalid.join(", "),
                valid: BodyField::valid_names().join(", "),
            });
        }

        Ok(Self { fields: parsed })
    }

    pub fn includes(&self, field: BodyField) -> bool {
        self.fields.contains(&field)
    }
}

impl BodyField {
    pub fn valid_names() -> Vec<&'static str> {
        vec![
            "event",
            "repository",
            "action",
            "prior_status",
            "ref",
            "commit",
            "sender",
            "url",
            "delivery",
        ]
    }
}

impl std::str::FromStr for BodyField {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "event" => Ok(Self::Event),
            "repository" => Ok(Self::Repository),
            "action" => Ok(Self::Action),
            "prior_status" => Ok(Self::PriorStatus),
            "ref" => Ok(Self::Ref),
            "commit" => Ok(Self::Commit),
            "sender" => Ok(Self::Sender),
            "url" => Ok(Self::Url),
            "delivery" => Ok(Self::Delivery),
            _ => Err(()),
        }
    }
}

impl fmt::Display for BodyField {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Event => "event",
            Self::Repository => "repository",
            Self::Action => "action",
            Self::PriorStatus => "prior_status",
            Self::Ref => "ref",
            Self::Commit => "commit",
            Self::Sender => "sender",
            Self::Url => "url",
            Self::Delivery => "delivery",
        };
        formatter.write_str(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_exclusions() {
        let exclusions = BodyFieldExclusions::new(vec!["ref".into(), "commit".into()]).unwrap();
        assert!(exclusions.includes(BodyField::Ref));
        assert!(exclusions.includes(BodyField::Commit));
        assert!(!exclusions.includes(BodyField::Repository));
    }

    #[test]
    fn rejects_invalid_exclusions() {
        let error = BodyFieldExclusions::new(vec!["branch".into()]).unwrap_err();
        assert!(error.to_string().contains("branch"));
        assert!(error.to_string().contains("prior_status"));
    }
}
