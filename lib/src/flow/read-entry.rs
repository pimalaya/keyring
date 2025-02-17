//! # Read entry flow
//!
//! Module dedicated to the I/O-free [`ReadEntry`] flow.

use secrecy::SecretString;

use crate::{Io, State};

/// The I/O-free flow for reading a secret from a keyring entry.
#[derive(Clone, Debug)]
pub struct ReadEntry {
    state: State,
}

impl ReadEntry {
    /// Creates a new flow from the given keyring entry key.
    pub fn new(key: impl ToString) -> Self {
        Self {
            state: State::read(key),
        }
    }

    /// Takes the secret away from the inner I/O state.
    pub fn take_secret(&mut self) -> Option<SecretString> {
        self.state.take_secret()
    }
}

impl AsMut<State> for ReadEntry {
    fn as_mut(&mut self) -> &mut State {
        &mut self.state
    }
}

impl Iterator for ReadEntry {
    type Item = Io;

    fn next(&mut self) -> Option<Self::Item> {
        if self.state.secret.is_none() {
            Some(Io::Read)
        } else {
            None
        }
    }
}
