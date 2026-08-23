use super::finding::{Finding, Seed};

pub fn judge(
    result: &Result<plumb::vocabulary::Report, plumb::vocabulary::Refusal>,
) -> Vec<Finding> {
    use crate::catalog::rules::vocabulary::RETIRED_TERM_ABSENT;

    match result {
        Ok(report) => report
            .hits
            .iter()
            .map(|hit| {
                Finding::new(Seed::wrong(
                    &RETIRED_TERM_ABSENT,
                    format!(
                        "{} contains retired domain term {} in {}",
                        hit.path, hit.term, hit.surface
                    ),
                ))
            })
            .collect(),
        Err(error) => vec![Finding::new(Seed::blind(
            &RETIRED_TERM_ABSENT,
            error.to_string(),
        ))],
    }
}
