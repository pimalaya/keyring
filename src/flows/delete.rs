//! Module dedicated to the [`Delete`] entry I/O-free flow.

use crate::{Entry, Io, State};

/// I/O-free flow for deleting a keyring entry.
#[derive(Clone, Debug)]
pub struct Delete {
    state: State,
}

impl Delete {
    pub fn new(entry: Entry) -> Self {
        let state = State::new(entry);
        Self { state }
    }

    pub fn next(&mut self) -> Result<(), Io> {
        if let Some(true) = self.state.done.take() {
            Ok(())
        } else {
            Err(Io::Delete)
        }
    }
}

impl AsMut<State> for Delete {
    fn as_mut(&mut self) -> &mut State {
        &mut self.state
    }
}
