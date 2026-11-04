use super::model::Schedule;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub(super) struct Completed {
    pub index: usize,
    pub queued: Duration,
    pub elapsed: Duration,
    pub threads: usize,
    pub result: Result<(), String>,
}

pub(super) struct Queue {
    pub schedule: Schedule,
}

impl Queue {
    fn capacity(&self, next: usize, active: usize, available: usize) -> bool {
        self.schedule.order.get(next).is_some_and(|index| {
            active < self.schedule.slots && self.schedule.grants[*index] <= available
        })
    }

    pub fn run(
        &self,
        execute: &(impl Fn(usize, usize) -> Result<(), String> + Sync),
        observe: &impl Fn(&Completed),
    ) -> Result<Vec<Option<Completed>>, String> {
        self.schedule.validate()?;
        let origin = Instant::now();
        let (sender, receiver) = mpsc::channel::<Completed>();
        let mut completed = vec![None; self.schedule.order.len()];
        let mut next = 0;
        let mut active = 0;
        let mut available = self.schedule.cores;
        let mut failed = false;
        std::thread::scope(|scope| {
            loop {
                while let Ok(result) = receiver.try_recv() {
                    available += result.threads;
                    settle(result, &mut completed, observe, &mut failed);
                    active -= 1;
                }
                if !failed
                    && self.capacity(next, active, available)
                    && super::super::signal::received().is_none()
                {
                    let index = self.schedule.order[next];
                    let threads = self.schedule.grants[index];
                    available -= threads;
                    start(scope, execute, (index, threads, origin), sender.clone());
                    next += 1;
                    active += 1;
                    continue;
                }
                if active == 0 {
                    break;
                }
                let result = receiver
                    .recv()
                    .map_err(|_| "test workers ended without a completion record".to_string())?;
                available += result.threads;
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
        threads,
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
                let schedule = Schedule::weighted(cores, &vec![1; count]);
                schedule.validate().unwrap();
                assert!(schedule.slots <= count);
                assert_eq!(schedule.slots == 0, count == 0);
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
            schedule: Schedule::weighted(6, &[1; 4]),
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
            schedule: Schedule::weighted(2, &[1; 3]),
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
    fn unequal() {
        let schedule = Schedule::weighted(12, &[1, 900, 99]);
        assert_eq!(schedule.order, [1, 2, 0]);
        assert_eq!(schedule.grants, [1, 11, 1]);
        let queue = Queue { schedule };
        let barrier = Barrier::new(2);
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let completed = queue
            .run(
                &|index, threads| {
                    let current = active.fetch_add(threads, Ordering::SeqCst) + threads;
                    assert!(current <= 12);
                    peak.fetch_max(current, Ordering::SeqCst);
                    if index != 0 {
                        barrier.wait();
                    }
                    active.fetch_sub(threads, Ordering::SeqCst);
                    Ok(())
                },
                &|_| {},
            )
            .unwrap();
        assert_eq!(peak.load(Ordering::SeqCst), 12);
        assert_eq!(active.load(Ordering::SeqCst), 0);
        assert!(
            completed
                .iter()
                .all(|result| result.as_ref().unwrap().result.is_ok())
        );
        let cold = Schedule::cold(12, 3);
        assert_eq!(cold.slots, 1);
        assert_eq!(cold.grants, [12; 3]);
        let mut duplicate = Schedule::weighted(12, &[1; 3]);
        duplicate.order = vec![1; 3];
        assert!(duplicate.validate().is_err());
    }

    #[test]
    fn panic() {
        let queue = Queue {
            schedule: Schedule::weighted(1, &[1; 2]),
        };
        let completed = queue
            .run(&|_, _| panic!("deliberate worker panic"), &|_| {})
            .unwrap();
        assert!(completed[0].as_ref().unwrap().result.is_err());
        assert!(completed[1].is_none());
    }
}
