use secrecy::SecretString;

use crate::Entry;

/// The I/O state.
///
/// This struct represents the I/O state used by I/O connectors to
/// take and set data. It is usually held by flows themselves, and
/// serve as communication bridge between flows and I/O connectors.
#[derive(Clone, Debug)]
pub struct State {
    pub(crate) entry: Entry,
    pub(crate) secret: Option<SecretString>,
    pub(crate) done: Option<bool>,
}

impl State {
    pub fn new(entry: Entry) -> Self {
        Self {
            entry,
            secret: None,
            done: None,
        }
    }

    pub fn get_service(&self) -> &str {
        &self.entry.service
    }

    pub fn get_name(&self) -> &str {
        &self.entry.name
    }

    /// Puts the given secret into the inner I/O state.
    pub fn set_secret(&mut self, secret: impl Into<SecretString>) {
        self.secret.replace(secret.into());
        self.done();
    }

    /// Takes the secret away from the inner I/O state.
    pub fn take_secret(&mut self) -> Option<SecretString> {
        self.secret.take()
    }

    /// Takes the secret away from the inner I/O state.
    pub fn done(&mut self) {
        self.done.replace(true);
    }
}
