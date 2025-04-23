#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Entry {
    pub service: String,
    pub name: String,
}

impl Entry {
    pub fn new(name: impl ToString) -> Self {
        Self {
            service: env!("CARGO_CRATE_NAME").to_string(),
            name: name.to_string(),
        }
    }

    pub fn set_service(&mut self, service: impl ToString) {
        self.service = service.to_string();
    }

    pub fn with_service(mut self, service: impl ToString) -> Self {
        self.set_service(service);
        self
    }
}

impl TryFrom<Entry> for keyring::Entry {
    type Error = keyring::Error;

    fn try_from(entry: Entry) -> keyring::Result<Self> {
        keyring::Entry::new(&entry.service, &entry.name)
    }
}
