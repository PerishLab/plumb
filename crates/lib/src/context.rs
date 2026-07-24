use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;

struct Node {
    key: TypeId,
    value: Arc<dyn Any + Send + Sync>,
    parent: Option<Arc<Node>>,
}

#[derive(Clone, Default)]
pub struct Context {
    head: Option<Arc<Node>>,
}

thread_local! {
    static CURRENT: RefCell<Context> = RefCell::new(Context::default());
}

impl Context {
    pub fn root() -> Context {
        Context::default()
    }

    pub fn with<T: Send + Sync + 'static>(&self, value: T) -> Context {
        Context {
            head: Some(Arc::new(Node {
                key: TypeId::of::<T>(),
                value: Arc::new(value),
                parent: self.head.clone(),
            })),
        }
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>> {
        let mut held = &self.head;
        while let Some(node) = held {
            if node.key == TypeId::of::<T>() {
                return node.value.clone().downcast().ok();
            }
            held = &node.parent;
        }
        None
    }

    pub fn current() -> Context {
        CURRENT.with(|held| held.borrow().clone())
    }

    #[must_use]
    pub fn enter(&self) -> Scope {
        let prior = CURRENT.with(|held| held.replace(self.clone()));
        Scope {
            prior: Some(prior),
            stay: PhantomData,
        }
    }

    pub fn carry<F: Future>(&self, future: F) -> Carry<F> {
        Carry {
            context: self.clone(),
            future,
        }
    }
}

pub struct Scope {
    prior: Option<Context>,
    stay: PhantomData<*const ()>,
}

impl Drop for Scope {
    fn drop(&mut self) {
        if let Some(prior) = self.prior.take() {
            CURRENT.with(|held| held.replace(prior));
        }
    }
}

pin_project_lite::pin_project! {
    pub struct Carry<F> {
        context: Context,
        #[pin]
        future: F,
    }
}

impl<F: Future> Future for Carry<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, task: &mut std::task::Context<'_>) -> Poll<F::Output> {
        let this = self.project();
        let _scope = this.context.enter();
        this.future.poll(task)
    }
}
