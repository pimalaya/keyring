/// The keyring I/O request enum, emitted by flows and processed by
/// handlers.
///
/// This enum represents all the possible I/O requests that a keyring
/// flow can emit. I/O handlers should be able to handle all variants.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Io {
    /// I/O for reading a secret from a keyring entry.
    ///
    /// This variant requires I/O connectors to get the entry key
    /// using [`get_entry_ref`], to extract the associated secret from
    /// their inner keychain and to give it to the flow using
    /// [`set_secret`].
    ///
    /// [`get_entry_ref`]: crate::State::get_entry_ref
    /// [`set_secret`]: crate::State::set_secret
    Read,

    /// I/O for saving a keyring entry secret.
    ///
    /// This variant requires I/O connectors to get the entry key
    /// using [`get_entry_ref`], to take the secret away from the
    /// state via [`take_secret`] then to save it into their inner
    /// keychain.
    ///
    /// [`get_entry_ref`]: crate::State::get_entry_ref
    /// [`take_secret`]: crate::State::take_secret
    Write,

    /// I/O for deleting a keyring entry.
    ///
    /// This variant requires I/O connectors to get the entry key
    /// using [`get_entry_ref`] and to delete the matching entry from
    /// their inner keychain.
    ///
    /// [`get_entry_ref`]: crate::State::get_entry_ref
    Delete,
}
