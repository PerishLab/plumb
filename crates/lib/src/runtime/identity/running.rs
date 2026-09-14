use super::{Binding, Origin, Region, codec};
use std::sync::OnceLock;

struct Running {
    origin: Origin,
    binding: Option<Binding>,
    unbound: bool,
}

static RUNNING: OnceLock<Running> = OnceLock::new();

pub fn install(region: &Region, unbound: bool) -> Result<(), String> {
    let (origin, binding) = codec::Codec(&region.read()).decode()?;
    RUNNING
        .set(Running {
            origin,
            binding,
            unbound,
        })
        .map_err(|_| "process identity was initialized more than once".into())
}

pub fn ready() -> Result<(), String> {
    if RUNNING
        .get()
        .is_some_and(|held| held.unbound && held.binding.is_none())
    {
        Err("unbound build: Ship must bind a release marker before this executable is used".into())
    } else {
        Ok(())
    }
}

pub struct Reader<'a>(pub &'a str);
impl Reader<'_> {
    pub fn origin(&self) -> Option<&'static Origin> {
        Some(&self.running()?.origin)
    }

    pub fn binding(&self) -> Option<&'static Binding> {
        self.running()?.binding.as_ref()
    }

    pub fn version(&self) -> Option<&'static str> {
        let held = self.running()?;
        held.binding
            .as_ref()
            .map(|value| value.marker.as_str())
            .or(held.unbound.then_some("unbound"))
    }

    pub fn commit(&self) -> Option<&'static str> {
        self.running()?
            .binding
            .as_ref()
            .map(|value| value.commit.as_str())
    }

    pub fn channel(&self) -> Option<&'static str> {
        let held = self.running()?;
        held.binding
            .as_ref()
            .and_then(|value| value.channel().ok())
            .or(held.unbound.then_some("unbound"))
    }

    fn running(&self) -> Option<&'static Running> {
        RUNNING.get().filter(|held| held.origin.prefix == self.0)
    }
}
