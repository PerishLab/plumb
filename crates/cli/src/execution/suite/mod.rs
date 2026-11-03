mod manifest;
mod messages;
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
use model::{Group, Plan, Program};
use output::Output;
use queue::{Budget, Completed, Queue};
use std::path::Path;
use std::process::Command;

pub(crate) fn run(
    root: &Path,
    command: &(impl Fn() -> Result<Command, String> + Sync),
) -> Result<(), String> {
    let _held = super::signal::Hold::new()?;
    let runner = Runner {
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
        self.parallel(&plan.programs, &contexts)?;
        if plan.docs {
            self.invoke(&["test", "--locked", "--doc"], true)?;
        }
        super::signal::check()
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
        let original = messages::read(stream)?;
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
            let built = messages::read(&self.invoke(&arguments, false)?)?;
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

    fn parallel(&self, programs: &[Program], contexts: &[Context]) -> Result<(), String> {
        let queue = Queue {
            count: programs.len(),
            budget: Budget::new(self.cores, programs.len()),
        };
        let outputs = programs
            .iter()
            .map(|_| Output::new())
            .collect::<Result<Vec<_>, _>>()?;
        crate::diagnostic::event("test.plan", || {
            serde_json::json!({
                "slots":queue.budget.slots,"threads":queue.budget.threads,
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
        if results
            .iter()
            .any(|result| result.as_ref().is_none_or(|result| result.result.is_err()))
        {
            return Err("test programs failed; remaining programs were not started".into());
        }
        Ok(())
    }
}

fn describe(program: &Program) -> serde_json::Value {
    serde_json::json!({"package":program.name,"target":program.target.name,"executable":program.executable,
        "directory":program.directory,"features":program.features,"custom":program.custom})
}

fn report(result: &Completed, program: &Program, output: &Output) -> Result<(), String> {
    eprintln!(
        "guard/test: {}:{}: queued {:.3}s, ran {:.3}s, {}",
        program.name,
        program.target.name,
        result.queued.as_secs_f64(),
        result.elapsed.as_secs_f64(),
        if result.result.is_ok() {
            "completed"
        } else {
            "failed"
        }
    );
    output.render(true)?;
    if let Err(error) = &result.result {
        eprintln!("guard/test: {error}");
    }
    crate::diagnostic::event("test.completed", || {
        serde_json::json!({"program":describe(program),
        "queued_ms":result.queued.as_millis(),"elapsed_ms":result.elapsed.as_millis(),"result":result.result})
    });
    Ok(())
}
