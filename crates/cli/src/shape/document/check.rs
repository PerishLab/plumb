use crate::judge::catalog::rules::document as rule;
use crate::judge::finding::{Found, blind, wrong};
use crate::shape;

pub fn check(held: &shape::Shape) -> Found {
    use shape::document::Config;

    let mut found = Found::new();
    match &held.documents.config {
        Config::Outside => return found,
        Config::Wrong(error) => {
            found.push(wrong(&rule::SCHEMA, error));
            return found;
        }
        Config::Blind(error) => {
            found.push(blind(&rule::SCHEMA, error));
            return found;
        }
        Config::Held(_) => {}
    }
    if let Some(error) = &held.documents.blind {
        found.push(blind(&rule::ADMISSION, error));
        found.push(blind(&rule::EVIDENCE, error));
        found.push(blind(&rule::MAGNITUDE, error));
        return found;
    }
    if !held.documents.conflicts.is_empty() {
        found.push(wrong(
            &rule::ADMISSION,
            format!(
                "tracked Markdown lies outside the closed document surface: {}",
                held.documents.conflicts.join(", ")
            ),
        ));
    }
    for document in &held.documents.held {
        for error in &document.errors {
            found.push(wrong(
                &rule::SCHEMA,
                format!("document {} {error}", document.target),
            ));
        }
        for source in &document.sources {
            if let Some(error) = &source.error {
                found.push(blind(&rule::EVIDENCE, error));
                continue;
            }
            if source.expected.is_empty() {
                found.push(wrong(
                    &rule::EVIDENCE,
                    format!(
                        "document {} source {} is unaffirmed; read both sides and run plumb document",
                        document.target, source.path
                    ),
                ));
            } else if source.actual.as_deref() != Some(&source.expected) {
                found.push(wrong(
                    &rule::EVIDENCE,
                    format!(
                        "document {} was invalidated by source {}; read both sides and run plumb document",
                        document.target, source.path
                    ),
                ));
            }
        }
        if document.expected.is_empty() {
            found.push(wrong(
                &rule::EVIDENCE,
                format!(
                    "document {} target is unaffirmed; read both sides and run plumb document",
                    document.target
                ),
            ));
        } else if document.actual.as_deref() != Some(&document.expected) {
            found.push(wrong(
                &rule::EVIDENCE,
                format!(
                    "document {} or its binding topology changed; read both sides and run plumb document",
                    document.target
                ),
            ));
        }
        if let (Some(lines), Some(budget)) = (document.lines, document.budget)
            && lines > budget
        {
            found.push(wrong(
                &rule::MAGNITUDE,
                format!(
                    "document {} has {lines} Markdown lines, above {} budget {budget}",
                    document.target,
                    document.strategy.id()
                ),
            ));
        }
    }
    found
}
