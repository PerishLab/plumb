pub struct Course {
    dry: bool,
    said: Vec<String>,
}

impl Course {
    pub fn new(dry: bool) -> Self {
        Self {
            dry,
            said: Vec::new(),
        }
    }

    pub fn step<T>(
        &mut self,
        said: impl Into<String>,
        act: impl FnOnce() -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        self.said.push(said.into());
        if self.dry {
            return Ok(None);
        }
        act().map(Some)
    }

    pub fn dry(&self) -> bool {
        self.dry
    }

    pub fn plan(&self) -> String {
        self.said.join("\n")
    }
}
