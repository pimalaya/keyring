use std::{collections::HashMap, fmt, time::Duration};

use dbus::{
    arg::{PropMap, RefArg, Variant},
    blocking::{Connection, Proxy},
    Path,
};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

use crate::{
    event::KeyringEvent,
    state::{EntryState, KeyringState},
};

use super::{
    api::{OrgFreedesktopSecretCollection, OrgFreedesktopSecretItem, OrgFreedesktopSecretService},
    config::{DBUS_DEST, DBUS_PATH, ITEM_ATTRIBUTES, ITEM_LABEL, TIMEOUT},
    crypto::{Algorithm, ALGORITHM_PLAIN},
    encryption::EncryptionAlgorithm,
    flow::{CryptoIo, Io},
    state::{SecretServiceEntryState, SecretServiceEntryStateKind},
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot create D-Bus connection")]
    CreateSessionError(#[source] dbus::Error),
    #[error("cannot open D-Bus session")]
    OpenSessionError(#[source] dbus::Error),
    #[error("cannot get default secret service collection")]
    GetDefaultCollectionError(#[source] dbus::Error),
    #[error("cannot get session secret service collection")]
    GetSessionCollectionError(#[source] dbus::Error),
    #[error("cannot get secret service collections")]
    GetCollectionsError(#[source] dbus::Error),
    #[error("cannot create default secret service collection")]
    CreateDefaultCollectionError(#[source] dbus::Error),
    #[error("cannot create secret service collection item")]
    CreateItemError(#[source] dbus::Error),
    #[error("cannot search items from Secret Service using D-Bus")]
    SearchItemsError(#[source] dbus::Error),
    #[error("cannot get item matching {0}:{1} in Secret Service using D-Bus")]
    GetItemNotFoundError(String, String),
    #[error("cannot get secret from Secret Service using D-Bus")]
    GetSecretError(#[source] dbus::Error),
    #[error("cannot delete item from Secret Service using D-Bus")]
    DeleteItemError(#[source] dbus::Error),
}

pub type Result<T> = ::std::result::Result<T, Error>;

pub struct SecretService {
    connection: Connection,
    session_path: Path<'static>,
}

impl fmt::Debug for SecretService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretService")
            .field("session_path", &self.session_path)
            .finish_non_exhaustive()
    }
}

impl SecretService {
    pub fn connect(algorithm: EncryptionAlgorithm) -> Result<Self> {
        let connection = Connection::new_session().map_err(Error::CreateSessionError)?;
        let proxy = connection.with_proxy(DBUS_DEST, DBUS_PATH, TIMEOUT);

        let bytes_arg = Box::new(String::new()) as Box<dyn RefArg>;
        let (_, session_path) = proxy
            .open_session(algorithm.as_ref(), Variant(bytes_arg))
            .map_err(Error::OpenSessionError)?;

        Ok(Self {
            connection,
            session_path,
        })
    }

    pub fn get_default_collection(&self) -> Result<Collection<'_>> {
        let proxy = self.connection.with_proxy(DBUS_DEST, DBUS_PATH, TIMEOUT);
        let empty_path = Path::default();

        let collection_path = proxy
            .read_alias("default")
            .map_err(Error::GetDefaultCollectionError)?;

        if collection_path != empty_path {
            return Ok(Collection::new(self, collection_path));
        }

        let collection_path = proxy
            .read_alias("session")
            .map_err(Error::GetSessionCollectionError)?;

        if collection_path != empty_path {
            return Ok(Collection::new(self, collection_path));
        }

        let collections_path = proxy.collections().map_err(Error::GetCollectionsError)?;

        match collections_path.into_iter().next() {
            Some(collection_path) => Ok(Collection::new(self, collection_path)),
            None => {
                let props: PropMap = HashMap::from_iter(Some((
                    "org.freedesktop.Secret.Collection.Label".into(),
                    Variant(Box::new(String::from("default")) as Box<dyn RefArg>),
                )));

                let (collection_path, _prompt_path) = proxy
                    .create_collection(props, "default")
                    .map_err(Error::CreateDefaultCollectionError)?;

                let collection_path = if collection_path == empty_path {
                    // no creation path, so prompt
                    todo!()
                } else {
                    collection_path
                };

                Ok(Collection::new(self, collection_path))
            }
        }
    }
}

#[derive(Debug)]
pub struct Session {
    path: Path<'static>,
    algorithm: EncryptionAlgorithm,
}

#[derive(Debug)]
pub struct Collection<'a> {
    service: &'a SecretService,
    path: Path<'a>,
}

impl<'a> Collection<'a> {
    pub fn new(service: &'a SecretService, path: Path<'a>) -> Self {
        Self { service, path }
    }

    pub fn proxy(&self) -> Proxy<'_, &'a Connection> {
        self.service
            .connection
            .with_proxy(DBUS_DEST, &self.path, TIMEOUT)
    }

    pub fn find_item(
        &self,
        service: impl AsRef<str>,
        account: impl AsRef<str>,
    ) -> Result<Option<Item>> {
        let proxy = self.proxy();
        let attrs: HashMap<&str, &str> =
            HashMap::from_iter([("service", service.as_ref()), ("account", account.as_ref())]);

        let items_path = OrgFreedesktopSecretCollection::search_items(&proxy, attrs)
            .map_err(Error::SearchItemsError)?;

        match items_path.into_iter().next() {
            Some(path) => Ok(Some(Item::new(&self.service, path))),
            None => Ok(None),
        }
    }

    pub fn get_item(&self, service: impl AsRef<str>, account: impl AsRef<str>) -> Result<Item> {
        let service = service.as_ref();
        let account = account.as_ref();

        match self.find_item(service, account)? {
            Some(item) => Ok(item),
            None => {
                let service = service.to_owned();
                let account = account.to_owned();
                Err(Error::GetItemNotFoundError(service, account))
            }
        }
    }

    pub fn create_item(
        &self,
        service: impl ToString,
        account: impl ToString,
        secret: impl Into<SecretString>,
    ) -> Result<Item<'_>> {
        let secret = secret.into().expose_secret().as_bytes().to_vec();
        let label = Box::new(service.to_string() + ":" + &account.to_string());
        let attrs: Box<HashMap<String, String>> = Box::new(HashMap::from_iter([
            (String::from("service"), service.to_string()),
            (String::from("account"), account.to_string()),
        ]));

        let mut props: PropMap = PropMap::new();
        props.insert(ITEM_LABEL.into(), Variant(label));
        props.insert(ITEM_ATTRIBUTES.into(), Variant(attrs));

        let session_path = self.service.session_path.clone();
        let secret = (session_path, vec![], secret, "text/plain");
        let (item_path, _prompt_path) = self
            .proxy()
            .create_item(props, secret, true)
            .map_err(Error::CreateItemError)?;

        let item_path = if item_path == Path::default() {
            // no creation path, so prompt
            todo!()
        } else {
            item_path
        };

        Ok(Item::new(&self.service, item_path))
    }

    pub fn get_secret(
        &self,
        service: impl AsRef<str>,
        account: impl AsRef<str>,
    ) -> Result<SecretString> {
        let item = self.get_item(service, account)?;
        let (_path, _salt, secret, _mime) = item.get_secret()?;
        let secret = String::from_utf8(secret).unwrap();

        Ok(secret.into())
    }

    pub fn delete_item(&self, service: impl AsRef<str>, account: impl AsRef<str>) -> Result<()> {
        self.get_item(service, account)?.delete()?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Item<'a> {
    service: &'a SecretService,
    path: Path<'a>,
}

impl<'a> Item<'a> {
    pub fn new(service: &'a SecretService, path: Path<'a>) -> Self {
        Self { service, path }
    }

    pub fn proxy(&self) -> Proxy<'_, &'a Connection> {
        self.service
            .connection
            .with_proxy(DBUS_DEST, &self.path, TIMEOUT)
    }

    pub fn get_secret(&self) -> Result<(Path<'static>, Vec<u8>, Vec<u8>, String)> {
        let proxy = &self.proxy();
        let session_path = self.service.session_path.clone();
        OrgFreedesktopSecretItem::get_secret(proxy, session_path).map_err(Error::GetSecretError)
    }

    pub fn delete(&self) -> Result<Path> {
        let proxy = &self.proxy();
        OrgFreedesktopSecretItem::delete(proxy).map_err(Error::DeleteItemError)
    }
}

pub struct IoProcessor<'a> {
    service: String,
    account: String,
    ss: SecretService,
    item: Option<Item<'a>>,
}

impl IoProcessor<'_> {
    pub fn try_new(service: impl ToString, account: impl ToString) -> Result<Self> {
        Ok(Self {
            service: service.to_string(),
            account: account.to_string(),
            crypto: CryptoIoProcessor::try_new()?,
            ss: SecretService::connect()?,
            item: None,
        })
    }

    pub fn process(&mut self, io: Io) -> Result<()> {
        match io {
            Io::Read => {
                self.secret = self
                    .ss
                    .get_default_collection()?
                    .get_item(self.service.clone(), self.account.clone())?;
            }
            Io::Write => {
                self.ss.get_default_collection()?.create_item(
                    self.service.clone(),
                    self.account.clone(),
                    secret,
                )?;
            }
            Io::Delete => {
                self.ss
                    .get_default_collection()?
                    .delete_item(self.service.clone(), self.account.clone())?;
            }
            Io::Crypto(io) => self.crypto.process(io)?,
        }

        Ok(())
    }
}

pub struct CryptoIoProcessor<'a> {
    path: Path<'static>,
    algorithm: Algorithm,
    shared_key: Option<[u8; 16]>,
}

impl CryptoIoProcessor {
    pub fn try_new(proxy: Proxy<'_, &'_ Connection>, algorithm: Algorithm) -> Result<Self> {
        match algorithm {
            Algorithm::Plain => {
                let bytes_arg = Box::new(String::new()) as Box<dyn RefArg>;
                let (_, path) = proxy.open_session(ALGORITHM_PLAIN, Variant(bytes_arg))?;

                Ok(Self {
                    path,
                    algorithm,
                    shared_key: None,
                })
            }
            Algorithm::DhIetf1024Sha256Aes128CbcPkcs7 => {
                // crypto: create private and public key
                let keypair = crypto::Keypair::generate();

                // send our public key with algorithm to service
                let public_bytes = keypair.public.to_bytes_be();
                let bytes_arg = Variant(Box::new(public_bytes) as Box<dyn RefArg>);
                let (out, path) = p.open_session(ALGORITHM_DH, bytes_arg)?;

                // get service public key back and create shared key from it
                if let Some(server_public_key_bytes) = cast::<Vec<u8>>(&out.0) {
                    let shared_key = keypair.derive_shared(server_public_key_bytes);
                    Ok(Session {
                        path,
                        encryption,
                        #[cfg(any(feature = "crypto-rust", feature = "crypto-openssl"))]
                        shared_key: Some(shared_key),
                    })
                } else {
                    Err(Error::Parse)
                }
            }
        }
    }

    pub fn process(&mut self, io: CryptoIo) -> Result<()> {
        match io {
            CryptoIo::Encrypt => {
                todo!()
            }
            CryptoIo::Decrypt => {
                todo!()
            }
        }

        Ok(())
    }
}
