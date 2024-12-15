use dbus::Path;
use secrecy::SecretSlice;

use super::{crypto, Io};

pub trait Flow {
    fn clone_session_path(&self) -> Path<'static>;

    fn take_secret(&mut self) -> Option<SecretSlice<u8>>;
    fn take_salt(&mut self) -> Option<Vec<u8>>;

    fn give_secret(&mut self, secret: SecretSlice<u8>);
    fn give_salt(&mut self, salt: Vec<u8>);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadEntryState {
    Read,
    Decrypt,
}

#[derive(Clone, Debug)]
pub struct ReadEntryFlow {
    state: Option<ReadEntryState>,
    pub session_path: Path<'static>,
    pub secret: Option<SecretSlice<u8>>,
    pub salt: Option<Vec<u8>>,
}

impl ReadEntryFlow {
    pub fn new(session_path: Path<'static>) -> Self {
        Self {
            state: Some(ReadEntryState::Read),
            session_path,
            secret: None,
            salt: None,
        }
    }
}

impl Iterator for ReadEntryFlow {
    type Item = Io;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state.take()? {
            ReadEntryState::Read => {
                self.state.replace(ReadEntryState::Decrypt);
                Some(Io::Entry(crate::Io::Read))
            }
            ReadEntryState::Decrypt => Some(Io::Crypto(crypto::Io::Decrypt)),
        }
    }
}

impl Flow for ReadEntryFlow {
    fn clone_session_path(&self) -> Path<'static> {
        self.session_path.clone()
    }

    fn take_secret(&mut self) -> Option<SecretSlice<u8>> {
        self.secret.take()
    }

    fn take_salt(&mut self) -> Option<Vec<u8>> {
        self.salt.take()
    }

    fn give_secret(&mut self, secret: SecretSlice<u8>) {
        self.secret.replace(secret);
    }

    fn give_salt(&mut self, salt: Vec<u8>) {
        self.salt.replace(salt);
    }
}

impl crypto::Flow for ReadEntryFlow {
    fn take_secret(&mut self) -> Option<SecretSlice<u8>> {
        self.secret.take()
    }

    fn take_salt(&mut self) -> Option<Vec<u8>> {
        self.salt.take()
    }

    fn give_secret(&mut self, secret: SecretSlice<u8>) {
        self.secret.replace(secret);
    }

    fn give_salt(&mut self, salt: Vec<u8>) {
        self.salt.replace(salt);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WriteEntryState {
    Encrypt,
    Write,
}

#[derive(Clone, Debug)]
pub struct WriteEntryFlow {
    state: Option<WriteEntryState>,
    pub session_path: Path<'static>,
    pub secret: Option<SecretSlice<u8>>,
    pub salt: Option<Vec<u8>>,
}

impl WriteEntryFlow {
    pub fn new(session_path: Path<'static>, secret: impl Into<SecretSlice<u8>>) -> Self {
        Self {
            state: Some(WriteEntryState::Encrypt),
            session_path,
            secret: Some(secret.into()),
            salt: None,
        }
    }
}

impl Iterator for WriteEntryFlow {
    type Item = Io;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state.take()? {
            WriteEntryState::Encrypt => {
                self.state.replace(WriteEntryState::Write);
                Some(Io::Crypto(crypto::Io::Encrypt))
            }
            WriteEntryState::Write => Some(Io::Entry(crate::Io::Write)),
        }
    }
}

impl Flow for WriteEntryFlow {
    fn clone_session_path(&self) -> Path<'static> {
        self.session_path.clone()
    }

    fn take_secret(&mut self) -> Option<SecretSlice<u8>> {
        self.secret.take()
    }

    fn take_salt(&mut self) -> Option<Vec<u8>> {
        self.salt.take()
    }

    fn give_secret(&mut self, secret: SecretSlice<u8>) {
        self.secret.replace(secret);
    }

    fn give_salt(&mut self, salt: Vec<u8>) {
        self.salt.replace(salt);
    }
}

impl crypto::Flow for WriteEntryFlow {
    fn take_secret(&mut self) -> Option<SecretSlice<u8>> {
        self.secret.take()
    }

    fn take_salt(&mut self) -> Option<Vec<u8>> {
        self.salt.take()
    }

    fn give_secret(&mut self, secret: SecretSlice<u8>) {
        self.secret.replace(secret);
    }

    fn give_salt(&mut self, salt: Vec<u8>) {
        self.salt.replace(salt);
    }
}

// #[derive(Clone, Debug)]
// pub struct DeleteEntryFlow {
//     delete: Option<EntryIo>,
// }

// impl Default for DeleteEntryFlow {
//     fn default() -> Self {
//         Self {
//             delete: Some(EntryIo::Delete),
//         }
//     }
// }

// impl Iterator for DeleteEntryFlow {
//     type Item = Io;

//     fn next(&mut self) -> Option<Self::Item> {
//         Some(Io::Entry(self.delete.take()?))
//     }
// }
