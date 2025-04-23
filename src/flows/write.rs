//! Module dedicated to the [`Write`] entry I/O-free flow.

use secrecy::SecretString;

use crate::{Entry, Io, State};

/// The I/O-free flow for saving a keyring entry secret.
#[derive(Clone, Debug)]
pub struct Write {
    state: State,
}

impl Write {
    /// Creates a new flow from the given keyring entry key.
    pub fn new(entry: Entry, secret: impl Into<SecretString>) -> Self {
        let mut state = State::new(entry);
        state.set_secret(secret);
        Self { state }
    }

    pub fn next(&mut self) -> Result<(), Io> {
        if let Some(true) = self.state.done.take() {
            Ok(())
        } else {
            Err(Io::Write)
        }
    }
}

impl AsMut<State> for Write {
    fn as_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
