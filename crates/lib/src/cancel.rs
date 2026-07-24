use crate::context::Context;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};

struct Token {
    fired: AtomicBool,
    wakers: Mutex<Vec<Waker>>,
    parent: Option<Arc<Token>>,
}

impl Token {
    fn mint(parent: Option<Arc<Token>>) -> Arc<Token> {
        Arc::new(Token {
            fired: AtomicBool::new(false),
            wakers: Mutex::new(Vec::new()),
            parent,
        })
    }

    fn fired(&self) -> bool {
        let mut held = Some(self);
        while let Some(token) = held {
            if token.fired.load(Ordering::Acquire) {
                return true;
            }
            held = token.parent.as_deref();
        }
        false
    }

    fn watch(&self, waker: &Waker) {
        let mut held = Some(self);
        while let Some(token) = held {
            let mut wakers = token.wakers.lock().unwrap();
            if !wakers.iter().any(|seen| seen.will_wake(waker)) {
                wakers.push(waker.clone());
            }
            held = token.parent.as_deref();
        }
    }
}

#[derive(Clone)]
pub struct Cancel {
    token: Arc<Token>,
}

impl Cancel {
    pub fn root() -> Cancel {
        Cancel {
            token: Token::mint(None),
        }
    }

    pub fn fork(&self) -> Cancel {
        Cancel {
            token: Token::mint(Some(self.token.clone())),
        }
    }

    pub fn child() -> (Cancel, Context) {
        let current = Context::current();
        let cancel = match current.get::<Cancel>() {
            Some(parent) => parent.fork(),
            None => Cancel::root(),
        };
        let context = current.with(cancel.clone());
        (cancel, context)
    }

    pub fn cancel(&self) {
        self.token.fired.store(true, Ordering::Release);
        let drained = std::mem::take(&mut *self.token.wakers.lock().unwrap());
        for waker in drained {
            waker.wake();
        }
    }

    pub fn cancelled(&self) -> bool {
        self.token.fired()
    }

    pub fn done(&self) -> Done {
        Done {
            token: self.token.clone(),
        }
    }
}

pub struct Done {
    token: Arc<Token>,
}

impl Future for Done {
    type Output = ();

    fn poll(self: Pin<&mut Self>, task: &mut std::task::Context<'_>) -> Poll<()> {
        if self.token.fired() {
            return Poll::Ready(());
        }
        self.token.watch(task.waker());
        if self.token.fired() {
            return Poll::Ready(());
        }
        Poll::Pending
    }
}
