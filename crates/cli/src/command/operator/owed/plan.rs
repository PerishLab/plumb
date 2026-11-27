use super::super::consign::Consign;
use super::Obligation;
use crate::shape::release::Spec;
use plumb::land::rejoin::{Stable, latest, tags};
use std::path::PathBuf;

#[derive(Default, Eq, PartialEq)]
pub(super) struct State {
    pub distributed: bool,
    pub rejoined: bool,
    pub changelog: bool,
    pub skill: bool,
    pub closed: bool,
    pub wants: bool,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct Plan {
    pub changelog: Option<PathBuf>,
    pub skill: Option<PathBuf>,
    pub close: bool,
}

impl State {
    pub(super) fn from(spec: &Spec, statuses: &[(&Obligation, bool)]) -> Self {
        let mut state = Self {
            wants: spec.skill,
            skill: !spec.skill,
            ..Self::default()
        };
        for (obligation, done) in statuses {
            match obligation.name {
                "its distribution completed" => state.distributed = *done,
                "its changes held by main" => state.rejoined = *done,
                "its changelog lodged on Depot" => state.changelog = *done,
                "its skill lodged on Depot" => state.skill = *done,
                "its release line closed" => state.closed = *done,
                _ => {}
            }
        }
        state
    }

    pub(super) fn plan(
        &self,
        marker: &str,
        changelog: Option<PathBuf>,
        skill: Option<PathBuf>,
    ) -> Result<Plan, String> {
        if !self.distributed {
            return Err(format!(
                "stable {marker} has not completed distribution; run plumb ship dispatch --marker {marker} --watch"
            ));
        }
        if !self.rejoined {
            return Err(format!(
                "stable {marker} is not yet held by main; run plumb release rejoin"
            ));
        }
        if skill.is_some() && !self.wants {
            return Err("repository declares no skill to consign".into());
        }
        let notes = (!self.changelog).then_some(changelog).flatten();
        let brief = (self.wants && !self.skill).then_some(skill).flatten();
        let noted = self.changelog || notes.is_some();
        let briefed = self.skill || brief.is_some();
        Ok(Plan {
            changelog: notes,
            skill: brief,
            close: noted && briefed && !self.closed,
        })
    }
}

impl Plan {
    pub(super) fn steps(&self, marker: &str) -> Vec<String> {
        let mut list = Vec::new();
        for (kind, dir) in [("changelog", &self.changelog), ("skill", &self.skill)] {
            if let Some(path) = dir {
                list.push(format!(
                    "plumb depot consign --version {marker} --kind {kind} --dir {}",
                    path.display()
                ));
            }
        }
        if self.close {
            list.push(format!("plumb release close --version {marker}"));
        }
        list
    }

    pub(super) fn jobs(&self, marker: &str) -> Vec<Consign> {
        [("changelog", &self.changelog), ("skill", &self.skill)]
            .into_iter()
            .filter_map(|(kind, dir)| {
                dir.as_ref().map(|path| Consign {
                    version: marker.to_string(),
                    kind: kind.to_string(),
                    dir: Some(path.clone()),
                    unwatched: false,
                    dry: false,
                })
            })
            .collect()
    }
}

pub(super) fn target(listing: &str, version: Option<&str>) -> Result<Stable, String> {
    match version {
        Some(raw) => {
            let marker = crate::command::changelog::stamped(raw);
            tags(listing)
                .into_iter()
                .find_map(|(name, commit)| {
                    (name == marker).then(|| Stable {
                        marker: name.into(),
                        commit: commit.into(),
                    })
                })
                .ok_or_else(|| format!("remote holds no stable marker {marker}"))
        }
        None => latest(tags(listing))
            .ok_or_else(|| "remote holds no stable marker to settle".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Plan, State, target};
    use std::path::PathBuf;

    #[test]
    fn planning() {
        let complete = State {
            distributed: true,
            rejoined: true,
            changelog: true,
            skill: true,
            closed: true,
            wants: true,
        };
        assert_eq!(
            complete
                .plan("v1.0.0", Some(PathBuf::from("a")), Some(PathBuf::from("b")))
                .expect("noop"),
            Plan::default()
        );
        let undone = State {
            distributed: false,
            rejoined: true,
            ..State::default()
        };
        assert!(
            undone
                .plan("v1.0.0", None, None)
                .expect_err("undistributed")
                .contains("ship dispatch")
        );
        let unmerged = State {
            distributed: true,
            rejoined: false,
            ..State::default()
        };
        assert!(
            unmerged
                .plan("v1.0.0", None, None)
                .expect_err("unrejoined")
                .contains("release rejoin")
        );
        let pending = State {
            distributed: true,
            rejoined: true,
            changelog: false,
            skill: false,
            closed: false,
            wants: true,
        };
        assert_eq!(
            pending
                .plan(
                    "v1.0.0",
                    Some(PathBuf::from("notes")),
                    Some(PathBuf::from("brief"))
                )
                .expect("both"),
            Plan {
                changelog: Some(PathBuf::from("notes")),
                skill: Some(PathBuf::from("brief")),
                close: true,
            }
        );
        assert_eq!(
            pending
                .plan("v1.0.0", Some(PathBuf::from("notes")), None)
                .expect("partial"),
            Plan {
                changelog: Some(PathBuf::from("notes")),
                skill: None,
                close: false,
            }
        );
        let noskill = State {
            distributed: true,
            rejoined: true,
            changelog: true,
            skill: true,
            closed: false,
            wants: false,
        };
        assert!(
            noskill
                .plan("v1.0.0", None, Some(PathBuf::from("brief")))
                .expect_err("no skill")
                .contains("no skill")
        );
        assert_eq!(
            noskill.plan("v1.0.0", None, None).expect("close only"),
            Plan {
                changelog: None,
                skill: None,
                close: true,
            }
        );
    }

    #[test]
    fn resolves() {
        let listing =
            "aaa\trefs/tags/v0.1.0^{}\nbbb\trefs/tags/v0.2.0^{}\nccc\trefs/tags/v0.3.0-rc.1^{}\n";
        let latest = target(listing, None).expect("latest stable");
        assert_eq!(latest.marker, "v0.2.0");
        assert_eq!(latest.commit, "bbb");
        let explicit = target(listing, Some("0.1.0")).expect("explicit");
        assert_eq!(explicit.marker, "v0.1.0");
        assert!(target(listing, Some("9.9.9")).is_err());
    }
}
