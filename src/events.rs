use std::collections::HashSet;

use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Clone)]
pub struct EventFilter {
    event_types: HashSet<&'static str>,
    explicit_wire_events: HashSet<&'static str>,
    fallback_wire_events: HashSet<&'static str>,
}

#[derive(Debug, Clone)]
pub struct EventNames(Vec<String>);

#[derive(Debug, Error)]
pub enum EventConfigError {
    #[error("no Forgejo events configured; set FORGEJO_EVENTS or forgejo.events")]
    Empty,
    #[error("invalid Forgejo event names: {invalid}. valid names: {valid}")]
    Invalid { invalid: String, valid: String },
}

type EventSpec = (&'static str, &'static str);

impl EventNames {
    pub fn new(names: Vec<String>) -> Self {
        Self(names)
    }
}

impl EventFilter {
    pub fn new(names: EventNames) -> Result<Self, EventConfigError> {
        let names = names
            .0
            .into_iter()
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())
            .collect::<Vec<_>>();
        if names.is_empty() {
            return Err(EventConfigError::Empty);
        }

        let mut invalid = Vec::new();
        let mut event_types = HashSet::new();
        let mut explicit_wire_events = HashSet::new();
        let mut fallback_wire_events = HashSet::new();
        for name in names {
            match lookup(&name) {
                Some(Match::Type(spec)) => {
                    event_types.insert(spec.0);
                    if !spec.1.is_empty() {
                        fallback_wire_events.insert(spec.1);
                    }
                    if !spec.1.is_empty() && spec.0 != spec.1 {
                        warn!(
                            event_type = spec.0,
                            wire_event = spec.1,
                            "Forgejo sends this event type under a broader X-Forgejo-Event"
                        );
                    }
                }
                Some(Match::Wire(wire_event)) => {
                    explicit_wire_events.insert(wire_event);
                }
                None => invalid.push(name),
            }
        }

        if !invalid.is_empty() {
            return Err(EventConfigError::Invalid {
                invalid: invalid.join(", "),
                valid: valid_names().join(", "),
            });
        }

        info!(
            event_types = ?event_types,
            explicit_wire_events = ?explicit_wire_events,
            fallback_wire_events = ?fallback_wire_events,
            "subscribed Forgejo events configured"
        );
        Ok(Self {
            event_types,
            explicit_wire_events,
            fallback_wire_events,
        })
    }

    pub fn is_subscribed(&self, event: &str, event_type: Option<&str>) -> bool {
        match event_type {
            Some(event_type) => {
                self.event_types.contains(event_type) || self.explicit_wire_events.contains(event)
            }
            None => {
                self.explicit_wire_events.contains(event)
                    || self.fallback_wire_events.contains(event)
            }
        }
    }

    pub fn is_known_incoming(&self, event: &str, event_type: Option<&str>) -> bool {
        let event_known = event.is_empty() || SPECS.iter().any(|spec| spec.1 == event);
        let type_known =
            event_type.is_none_or(|event_type| SPECS.iter().any(|spec| spec.0 == event_type));
        event_known && type_known
    }
}

enum Match {
    Type(EventSpec),
    Wire(&'static str),
}

fn lookup(name: &str) -> Option<Match> {
    SPECS
        .iter()
        .copied()
        .find(|spec| spec.0 == name)
        .map(Match::Type)
        .or_else(|| {
            SPECS
                .iter()
                .find(|spec| !spec.1.is_empty() && spec.1 == name)
                .map(|spec| Match::Wire(spec.1))
        })
}

fn valid_names() -> Vec<&'static str> {
    let mut names = SPECS
        .iter()
        .flat_map(|spec| [spec.0, spec.1])
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    names.sort_unstable();
    names.dedup();
    names
}

const SPECS: &[EventSpec] = &[
    ("create", "create"),
    ("delete", "delete"),
    ("fork", "fork"),
    ("push", "push"),
    ("issues", "issues"),
    ("issue_assign", "issues"),
    ("issue_label", "issues"),
    ("issue_milestone", "issues"),
    ("issue_comment", "issue_comment"),
    ("pull_request", "pull_request"),
    ("pull_request_assign", "pull_request"),
    ("pull_request_label", "pull_request"),
    ("pull_request_milestone", "pull_request"),
    ("pull_request_comment", "issue_comment"),
    ("pull_request_review_approved", "pull_request_approved"),
    ("pull_request_review_rejected", "pull_request_rejected"),
    ("pull_request_review_comment", "pull_request_comment"),
    ("pull_request_sync", "pull_request"),
    ("pull_request_review_request", "pull_request"),
    ("wiki", "wiki"),
    ("repository", "repository"),
    ("release", "release"),
    ("package", ""),
    ("schedule", ""),
    ("workflow_dispatch", ""),
    ("action_run_failure", "action_run_failure"),
    ("action_run_recover", "action_run_recover"),
    ("action_run_success", "action_run_success"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_event_type_names() {
        let filter = EventFilter::new(EventNames::new(vec!["action_run_failure".into()])).unwrap();
        assert!(filter.is_subscribed("action_run_failure", Some("action_run_failure")));
    }

    #[test]
    fn accepts_wire_event_names() {
        let filter =
            EventFilter::new(EventNames::new(vec!["pull_request_approved".into()])).unwrap();
        assert!(filter.is_subscribed(
            "pull_request_approved",
            Some("pull_request_review_approved")
        ));
    }

    #[test]
    fn specific_event_type_does_not_match_sibling_with_same_wire_event() {
        let filter = EventFilter::new(EventNames::new(vec!["issue_assign".into()])).unwrap();
        assert!(!filter.is_subscribed("issues", Some("issues")));
        assert!(filter.is_subscribed("issues", Some("issue_assign")));
    }

    #[test]
    fn rejects_empty_events() {
        assert!(matches!(
            EventFilter::new(EventNames::new(vec!["".into()])),
            Err(EventConfigError::Empty)
        ));
    }
}
