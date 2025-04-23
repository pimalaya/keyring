//! Module dedicated to the [`Read`] entry I/O-free flow.

use secrecy::SecretString;

use crate::{Entry, Io, State};

/// I/O-free flow for reading a secret from a keyring entry.
#[derive(Clone, Debug)]
pub struct Read {
    state: State,
}

impl Read {
    pub fn new(entry: Entry) -> Self {
        let state = State::new(entry);
        Self { state }
    }

    pub fn next(&mut self) -> Result<SecretString, Io> {
        match self.state.take_secret() {
            Some(secret) => Ok(secret),
            None => Err(Io::Read),
        }
    }
}

impl AsMut<State> for Read {
    fn as_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
