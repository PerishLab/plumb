mod history;
mod manifest;
mod model;
mod output;
mod queue;
mod selection;

#[cfg(all(test, target_os = "linux"))]
mod cancel;
#[cfg(test)]
mod runtime;
#[cfg(test)]
mod tests;

use super::capture::{Context, Image, Session};
use history::History;
use model::{Group, Plan, Program, Schedule};
use output::{Output, describe, report};
use queue::{Completed, Queue};
use std::path::Path;
use std::process::Command;

pub(crate) fn run(
    root: &Path,
    command: &(impl Fn() -> Result<Command, String> + Sync),
) -> Result<(), String> {
    let _held = super::signal::Hold::new()?;
    let home = crate::command::cache::home()?;
    let runner = Runner {
        home: &home,
        command,
        carrier: &[],
        cores: std::thread::available_parallelism().map_or(1, usize::from),
    };
    if let Some(reason) = super::capture::inspect(root, &(command)()?)? {
        return runner.fallback(&reason);
    }
    let image = match Image::new() {
        Ok(image) => image,
        Err(error) => {
            return runner.fallback(&format!("cannot retain this Guard runner image: {error}"));
        }
    };
    let carrier = image.carrier()?;
    Runner {
        carrier: &carrier,
        ..runner
    }
    .execute(root)
}

struct Runner<'a, F> {
    home: &'a Path,
    command: &'a F,
    cores: usize,
    carrier: &'a [String],
}

impl<F: Fn() -> Result<Command, String> + Sync> Runner<'_, F> {
    fn execute(&self, root: &Path) -> Result<(), String> {
        if let Some(reason) = super::capture::inspect(root, &(self.command)()?)? {
            return self.fallback(&reason);
        }
        let stream = self.invoke(
            &[
                "test",
                "--locked",
                "--no-run",
                "--message-format=json-render-diagnostics",
            ],
            false,
        )?;
        let metadata = self.invoke(
            &["metadata", "--locked", "--no-deps", "--format-version=1"],
            false,
        )?;
        let plan = Plan::read(root, &metadata, &stream)?;
        if !plan.fallback.is_empty() {
            return self.fallback(&plan.fallback.join("; "));
        }
        let groups = match plan.groups(&metadata) {
            Ok(groups) => groups,
            Err(reason) => return self.fallback(&reason),
        };
        if let Some(reason) = self.qualify(&groups, &stream)? {
            return self.fallback(&reason);
        }
        let session = Session::new(
            plan.programs
                .iter()
                .map(|program| (program.executable.clone(), program.directory.clone())),
            self.carrier,
        )?;
        let contexts = match self.capture(&groups, &session) {
            Ok(contexts) => contexts,
            Err(reason) => {
                super::signal::check()?;
                return self.fallback(&format!("launch-context capture refused: {reason}"));
            }
        };
        let (history, schedule) = History::prepare(root, self.home, self.cores, &plan.programs);
        let results = self.parallel(&plan.programs, &contexts, schedule)?;
        if plan.docs {
            self.invoke(&["test", "--locked", "--doc"], true)?;
        }
        super::signal::check()?;
        if let Some(history) = history
            && let Err(error) = history.record(&results)
        {
            history::refused("result retained without new timings", &error);
        }
        Ok(())
    }

    fn invoke(&self, arguments: &[&str], visible: bool) -> Result<String, String> {
        crate::diagnostic::event("test.cargo", || serde_json::json!({"arguments":arguments}));
        let output = Output::new()?;
        let mut command = (self.command)()?;
        command.args(arguments);
        let result = output.execute(command);
        output.render(visible)?;
        let status = result?;
        super::signal::check()?;
        if !status.success() {
            return Err(format!(
                "cargo {} failed with {status}",
                arguments.join(" ")
            ));
        }
        if visible {
            Ok(String::new())
        } else {
            output.text()
        }
    }

    fn fallback(&self, reason: &str) -> Result<(), String> {
        eprintln!("guard/test: serial Cargo fallback: {reason}");
        crate::diagnostic::event("test.fallback", || serde_json::json!({"reason":reason}));
        self.invoke(&["test", "--locked"], true).map(|_| ())
    }

    fn qualify(&self, groups: &[Group<'_>], stream: &str) -> Result<Option<String>, String> {
        let original = selection::read(stream)?;
        for group in groups {
            let mut arguments = vec![
                "test",
                "--locked",
                "--no-run",
                "--jobs",
                "1",
                "--message-format=json-render-diagnostics",
            ];
            arguments.extend(group.arguments.iter().map(String::as_str));
            let built = selection::read(&self.invoke(&arguments, false)?)?;
            if built.iter().any(|artifact| {
                !artifact.fresh || !original.iter().any(|original| artifact.same(original))
            }) {
                return Ok(Some(format!(
                    "{} changes the original Cargo compilation context",
                    group.arguments.join(" ")
                )));
            }
            let actual = built
                .iter()
                .filter(|artifact| artifact.profile.test)
                .filter_map(|artifact| artifact.executable.as_ref())
                .map(|path| {
                    path.canonicalize()
                        .map_err(|error| format!("cannot resolve captured test program: {error}"))
                })
                .collect::<Result<std::collections::BTreeSet<_>, _>>()?;
            let expected = group
                .programs
                .iter()
                .map(|program| program.executable.clone())
                .collect::<std::collections::BTreeSet<_>>();
            if actual != expected {
                return Ok(Some(format!(
                    "{} changes the original Cargo test set",
                    group.arguments.join(" ")
                )));
            }
        }
        Ok(None)
    }

    fn capture(&self, groups: &[Group<'_>], session: &Session) -> Result<Vec<Context>, String> {
        let configuration = session.configuration(self.carrier)?;
        for group in groups {
            let mut arguments = vec![
                "test",
                "--locked",
                "--jobs",
                "1",
                "--config",
                &configuration,
            ];
            arguments.extend(group.arguments.iter().map(String::as_str));
            self.invoke(&arguments, false)?;
        }
        let contexts = session.contexts()?;
        crate::diagnostic::event(
            "test.captured",
            || serde_json::json!({"programs":contexts.len(),"executed":false}),
        );
        Ok(contexts)
    }

    fn parallel(
        &self,
        programs: &[Program],
        contexts: &[Context],
        schedule: Schedule,
    ) -> Result<Vec<Option<Completed>>, String> {
        let queue = Queue { schedule };
        let outputs = programs
            .iter()
            .map(|_| Output::new())
            .collect::<Result<Vec<_>, _>>()?;
        crate::diagnostic::event("test.plan", || {
            serde_json::json!({
                "slots":queue.schedule.slots,"cores":queue.schedule.cores,"grants":queue.schedule.grants,"order":queue.schedule.order,
                "programs":programs.iter().map(describe).collect::<Vec<_>>()
            })
        });
        let rendering = std::cell::RefCell::new(Vec::new());
        let results = queue.run(
            &|index, threads| {
                let status = outputs[index].execute(contexts[index].command(threads)?)?;
                if status.success() {
                    Ok(())
                } else {
                    Err(format!("test program failed with {status}"))
                }
            },
            &|result| {
                if let Err(error) = report(result, &programs[result.index], &outputs[result.index])
                {
                    rendering.borrow_mut().push(error);
                }
            },
        )?;
        for (index, result) in results.iter().enumerate() {
            if result.is_none() {
                eprintln!(
                    "guard/test: not started: {}:{}",
                    programs[index].name, programs[index].target.name
                );
            }
        }
        super::signal::check()?;
        if !rendering.borrow().is_empty() {
            return Err(rendering.borrow().join("; "));
        }
        let failed = results
            .iter()
            .flatten()
            .any(|result| result.result.is_err());
        let skipped = results.iter().any(Option::is_none);
        match (failed, skipped) {
            (true, true) => Err("test programs failed; remaining programs were not started".into()),
            (true, false) => Err("test programs failed".into()),
            (false, true) => Err("remaining test programs were not started".into()),
            (false, false) => Ok(results),
        }
    }
}
