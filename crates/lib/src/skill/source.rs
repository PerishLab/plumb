use super::{Ask, Done, Error, Kit, Record, Report, fetch};

pub struct Depot<'a> {
    kit: &'a Kit,
    source: &'a str,
    product: &'a str,
    version: &'a str,
}

pub(super) trait Source {
    fn resolve(&self, ask: &Ask) -> Result<fetch::Grant, Error>;
    fn admits(&self, channel: &str) -> bool;
}

pub(super) struct Release<'a>(pub &'a str);

impl Kit {
    pub fn depot<'a>(&'a self, source: &'a str, product: &'a str, version: &'a str) -> Depot<'a> {
        Depot {
            kit: self,
            source,
            product,
            version,
        }
    }
}

impl Depot<'_> {
    pub fn channel(&self) -> String {
        fetch::channel(self.version)
    }

    pub fn install(&self, ask: &Ask) -> Result<Done, Error> {
        self.kit.installing(self, ask)
    }

    pub fn upgrade(&self, ask: &Ask) -> Result<Done, Error> {
        self.kit.upgrading(self, ask)
    }

    pub fn status(&self, ask: &Ask) -> Result<Report, Error> {
        self.kit.inspecting(self, ask)
    }

    pub fn stage(&self, ask: &Ask) -> Result<Done, Error> {
        self.kit.staging(self, ask)
    }

    pub fn list(&self) -> Result<Vec<Record>, Error> {
        self.kit.list()
    }

    pub fn uninstall(&self) -> Result<Done, Error> {
        self.kit.uninstall()
    }
}

impl Source for Release<'_> {
    fn resolve(&self, ask: &Ask) -> Result<fetch::Grant, Error> {
        fetch::resolve(self.0, &ask.channel, ask.version.as_deref())
    }

    fn admits(&self, channel: &str) -> bool {
        channel == "stable"
    }
}

impl Source for Depot<'_> {
    fn resolve(&self, ask: &Ask) -> Result<fetch::Grant, Error> {
        fetch::depot(
            self.source,
            self.product,
            self.version,
            fetch::Wanted {
                channel: &ask.channel,
                version: ask.version.as_deref(),
            },
        )
    }

    fn admits(&self, channel: &str) -> bool {
        channel == "stable" || fetch::belongs(channel, self.version)
    }
}
