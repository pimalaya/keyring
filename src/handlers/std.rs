//! Module dedicated to the standard, blocking stream I/O handler.

use keyring::{Entry, Error, Result};
use secrecy::ExposeSecret;

use crate::{Io, State};

/// The standard, blocking stream I/O handler.
///
/// Processes the [`Io`] request for the given flow, onto the
/// given stream.
pub fn handle(mut flow: impl AsMut<State>, io: Io) -> Result<()> {
    match io {
        Io::Read => read(flow.as_mut()),
        Io::Write => write(flow.as_mut()),
        Io::Delete => delete(flow.as_mut()),
    }
}

/// Processes the [`Io::Read`] request for the given flow's
/// [`State`], onto the given stream.
///
/// This function reads synchronously a chunk of bytes from the
/// given stream to the given state's read buffer, then set how
/// many bytes have been read.
pub fn read(state: &mut State) -> Result<()> {
    let entry = Entry::new(state.get_service(), state.get_name())?;
    let password = entry.get_password()?;
    state.set_secret(password);
    Ok(())
}

/// Processes the [`Io::Write`] request for the given flow's
/// [`State`], onto the given stream.
///
/// This function writes synchronously bytes to the given stream
/// from the given state's write buffer, then set how many bytes
/// have been written.
pub fn write(state: &mut State) -> Result<()> {
    let Some(secret) = state.take_secret() else {
        return Err(Error::NoEntry);
    };

    let entry = Entry::new(state.get_service(), state.get_name())?;
    let password = secret.expose_secret();
    entry.set_password(password)?;
    state.done();
    Ok(())
}

pub fn delete(state: &mut State) -> Result<()> {
    let entry = Entry::new(state.get_service(), state.get_name())?;
    entry.delete_credential()?;
    state.done();
    Ok(())
}
