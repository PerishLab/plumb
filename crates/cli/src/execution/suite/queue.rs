use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Budget {
    pub slots: usize,
    pub threads: usize,
}

impl Budget {
    pub fn new(cores: usize, count: usize) -> Self {
        let cores = cores.max(1);
        let slots = count.min(cores).min(cores.isqrt().max(2));
        Self {
            slots,
            threads: cores / slots.max(1),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Completed {
    pub index: usize,
    pub queued: Duration,
    pub elapsed: Duration,
    pub result: Result<(), String>,
}

pub(super) struct Queue {
    pub count: usize,
    pub budget: Budget,
}

impl Queue {
    fn capacity(&self, next: usize, active: usize) -> bool {
        next < self.count && active < self.budget.slots
    }

    pub fn run(
        &self,
        execute: &(impl Fn(usize, usize) -> Result<(), String> + Sync),
        observe: &impl Fn(&Completed),
    ) -> Result<Vec<Option<Completed>>, String> {
        if self.count != 0 && (self.budget.slots == 0 || self.budget.threads == 0) {
            return Err("test queue requires a nonzero program and thread budget".into());
        }
        let origin = Instant::now();
        let (sender, receiver) = mpsc::channel();
        let mut completed = vec![None; self.count];
        let mut next = 0;
        let mut active = 0;
        let mut failed = false;
        std::thread::scope(|scope| {
            loop {
                while let Ok(result) = receiver.try_recv() {
                    settle(result, &mut completed, observe, &mut failed);
                    active -= 1;
                }
                while !failed
                    && self.capacity(next, active)
                    && super::super::signal::received().is_none()
                {
                    start(
                        scope,
                        execute,
                        (next, self.budget.threads, origin),
                        sender.clone(),
                    );
                    next += 1;
                    active += 1;
                }
                if active == 0 {
                    break;
                }
                let result = receiver
                    .recv()
                    .map_err(|_| "test workers ended without a completion record".to_string())?;
                settle(result, &mut completed, observe, &mut failed);
                active -= 1;
            }
            Ok::<(), String>(())
        })?;
        Ok(completed)
    }
}

fn start<'scope, 'env>(
    scope: &'scope std::thread::Scope<'scope, 'env>,
    execute: &'scope (impl Fn(usize, usize) -> Result<(), String> + Sync),
    task: (usize, usize, Instant),
    sender: mpsc::Sender<Completed>,
) {
    scope.spawn(move || {
        let result = invoke(task.0, task.1, task.2, execute);
        let _ = sender.send(result);
    });
}

fn invoke(
    index: usize,
    threads: usize,
    origin: Instant,
    execute: &(impl Fn(usize, usize) -> Result<(), String> + Sync),
) -> Completed {
    let start = Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(index, threads)))
        .unwrap_or_else(|_| Err("test worker panicked; execution cleanup must be verified".into()));
    Completed {
        index,
        queued: start.duration_since(origin),
        elapsed: start.elapsed(),
        result,
    }
}

fn settle(
    result: Completed,
    completed: &mut [Option<Completed>],
    observe: &impl Fn(&Completed),
    failed: &mut bool,
) {
    *failed |= result.result.is_err();
    observe(&result);
    let index = result.index;
    completed[index] = Some(result);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Barrier, Mutex};

    #[test]
    fn budget() {
        for cores in 0..65 {
            for count in 0..65 {
                let budget = Budget::new(cores, count);
                assert!(budget.slots <= count);
                assert!(budget.slots * budget.threads <= cores.max(1));
                assert!(budget.threads > 0);
                assert_eq!(budget.slots == 0, count == 0);
            }
        }
    }

    #[test]
    fn parallel() {
        let barrier = Barrier::new(2);
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let seen = Mutex::new(Vec::new());
        let queue = Queue {
            count: 4,
            budget: Budget {
                slots: 2,
                threads: 3,
            },
        };
        let completed = queue
            .run(
                &|index, threads| {
                    assert_eq!(threads, 3);
                    let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(count, Ordering::SeqCst);
                    if index < 2 {
                        barrier.wait();
                    }
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                },
                &|result| seen.lock().unwrap().push(result.index),
            )
            .unwrap();
        assert_eq!(peak.load(Ordering::SeqCst), 2);
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert!(
            completed
                .iter()
                .all(|result| result.as_ref().unwrap().result.is_ok())
        );
        let mut seen = seen.into_inner().unwrap();
        seen.sort();
        assert_eq!(seen, [0, 1, 2, 3]);
    }

    #[test]
    fn failure() {
        let barrier = Barrier::new(2);
        let observed = AtomicBool::new(false);
        let drained = AtomicBool::new(false);
        let queue = Queue {
            count: 3,
            budget: Budget::new(2, 3),
        };
        let completed = queue
            .run(
                &|index, _| {
                    assert!(index < 2, "new work started after the observed failure");
                    barrier.wait();
                    if index == 0 {
                        return Err("deliberate failure".into());
                    }
                    let limit = Instant::now() + Duration::from_secs(5);
                    while !observed.load(Ordering::SeqCst) {
                        assert!(Instant::now() < limit);
                        std::thread::yield_now();
                    }
                    drained.store(true, Ordering::SeqCst);
                    Ok(())
                },
                &|result| {
                    if result.result.is_err() {
                        observed.store(true, Ordering::SeqCst);
                    }
                },
            )
            .unwrap();
        assert!(completed[0].as_ref().unwrap().result.is_err());
        assert!(completed[1].as_ref().unwrap().result.is_ok());
        assert!(completed[2].is_none());
        assert!(drained.load(Ordering::SeqCst));
    }

    #[test]
    fn panic() {
        let queue = Queue {
            count: 2,
            budget: Budget::new(1, 2),
        };
        let completed = queue
            .run(&|_, _| panic!("deliberate worker panic"), &|_| {})
            .unwrap();
        assert!(completed[0].as_ref().unwrap().result.is_err());
        assert!(completed[1].is_none());
    }
}
