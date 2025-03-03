use std::{
    collections::HashMap,
    fmt,
    sync::mpsc::{channel, TryRecvError},
    time::Duration,
};

use dbus::{
    arg::{cast, PropMap, RefArg, Variant},
    blocking::{Connection, Proxy},
    Message, Path,
};
use secrecy::{ExposeSecret, SecretSlice};
use thiserror::Error;
use tracing::warn;

use super::api::{
    OrgFreedesktopSecretCollection, OrgFreedesktopSecretItem, OrgFreedesktopSecretPrompt,
    OrgFreedesktopSecretPromptCompleted, OrgFreedesktopSecretService,
};
#[cfg(feature = "secret-service-crypto")]
use crate::secret_service::crypto::{
    self,
    sans_io::{PutSalt, TakeSalt, ALGORITHM_DH},
};
use crate::{
    sans_io::{GetKey, PutSecret, TakeSecret},
    secret_service::{
        crypto::sans_io::{Algorithm, ALGORITHM_PLAIN},
        dbus::Session,
        sans_io::{DBUS_DEST, DBUS_PATH, DEFAULT_TIMEOUT, ITEM_ATTRIBUTES, ITEM_LABEL},
    },
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot create Secret Service connection using D-Bus")]
    CreateSessionError(#[source] dbus::Error),
    #[error("cannot open Secret Service session using D-Bus")]
    OpenSessionError(#[source] dbus::Error),
    #[error("cannot parse Secret Service session output using D-Bus")]
    ParseSessionOutputError,

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
    #[error("cannot cast server public key to bytes")]
    CastServerPublicKeyToBytesError,
    #[error("cannot write empty secret into Secret Service entry using D-Bus")]
    WriteEmptySecretError,

    #[error("cannot prompt using D-Bus")]
    PromptError(#[source] dbus::Error),
    #[error("cannot prompt using D-Bus: match signal error")]
    PromptMatchSignalError(#[source] dbus::Error),
    #[error("cannot prompt using D-Bus: match stop error")]
    PromptMatchStopError(#[source] dbus::Error),
    #[error("cannot prompt using D-Bus: request timed out")]
    PromptTimeoutError,
    #[error("cannot prompt using D-Bus: prompt dismissed")]
    PromptDismissedError,
    #[error("cannot prompt using D-Bus: invalid prompt signal path")]
    ParsePromptPathError,
    #[error("cannot prompt using D-Bus: invalid prompt signal")]
    ParsePromptSignalError,

    #[cfg(feature = "secret-service-openssl-std")]
    #[error(transparent)]
    OpensslError(#[from] crypto::openssl::std::Error),
    #[cfg(feature = "secret-service-rust-crypto-std")]
    #[error(transparent)]
    RustCryptoError(#[from] crypto::rust_crypto::std::Error),
}

pub type Result<T> = ::std::result::Result<T, Error>;

pub struct SecretService {
    connection: Connection,
    session: Session,
}

impl fmt::Debug for SecretService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SecretService")
            .field("connection", &self.connection.unique_name())
            .field("session", &self.session)
            .finish()
    }
}

impl SecretService {
    pub fn connect(encryption: Algorithm) -> Result<Self> {
        let connection = Connection::new_session().map_err(Error::CreateSessionError)?;
        let proxy = connection.with_proxy(DBUS_DEST, DBUS_PATH, DEFAULT_TIMEOUT);
        let session = match encryption {
            Algorithm::Plain => {
                let input = Variant(Box::new(String::new()) as Box<dyn RefArg>);
                let (_, session_path) = proxy
                    .open_session(ALGORITHM_PLAIN, input)
                    .map_err(Error::OpenSessionError)?;
                Session::new_plain(session_path)
            }
            #[cfg(feature = "secret-service-crypto")]
            Algorithm::Dh(keypair) => {
                let input = Variant(Box::new(keypair.public.to_bytes_be()) as Box<dyn RefArg>);
                let (output, session_path) = proxy
                    .open_session(ALGORITHM_DH, input)
                    .map_err(Error::OpenSessionError)?;
                let output = cast::<Vec<u8>>(&output.0).ok_or(Error::ParseSessionOutputError)?;
                Session::new_dh(session_path, keypair, output.clone())
            }
        };

        Ok(Self {
            connection,
            session,
        })
    }

    pub fn get_default_collection(&self) -> Result<Collection<'_>> {
        let proxy = self
            .connection
            .with_proxy(DBUS_DEST, DBUS_PATH, DEFAULT_TIMEOUT);
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

                let (collection_path, prompt_path) = proxy
                    .create_collection(props, "default")
                    .map_err(Error::CreateDefaultCollectionError)?;

                let collection_path = if collection_path == empty_path {
                    // no creation path, so prompt
                    self.prompt(&prompt_path)?
                } else {
                    collection_path
                };

                Ok(Collection::new(self, collection_path))
            }
        }
    }

    fn prompt(&self, path: &Path) -> Result<Path<'static>> {
        let timeout = 5 * 60 * 60; // 5 min
        let proxy = self.connection.with_proxy(DBUS_DEST, path, DEFAULT_TIMEOUT);
        let (tx, rx) = channel::<Result<Path<'static>>>();

        let token = proxy
            .match_signal(
                move |signal: OrgFreedesktopSecretPromptCompleted, _: &Connection, _: &Message| {
                    let result = if signal.dismissed {
                        Err(Error::PromptDismissedError)
                    } else if let Some(first) = signal.result.as_static_inner(0) {
                        match cast::<Path<'_>>(first) {
                            Some(path) => Ok(path.clone().into_static()),
                            None => Err(Error::ParsePromptPathError),
                        }
                    } else {
                        Err(Error::ParsePromptSignalError)
                    };

                    if let Err(err) = tx.send(result) {
                        warn!(?err, "cannot send prompt result, exiting anyway")
                    }

                    false
                },
            )
            .map_err(Error::PromptMatchSignalError)?;

        proxy.prompt("").map_err(Error::PromptError)?;

        let mut result = Err(Error::PromptTimeoutError);

        for _ in 0..timeout {
            match self.connection.process(Duration::from_secs(1)) {
                Ok(false) => continue,
                Ok(true) => match rx.try_recv() {
                    Ok(res) => {
                        result = res;
                        break;
                    }
                    Err(TryRecvError::Empty) => continue,
                    Err(TryRecvError::Disconnected) => break,
                },
                _ => break,
            }
        }

        proxy
            .match_stop(token, true)
            .map_err(Error::PromptMatchStopError)?;

        result
    }
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
            .with_proxy(DBUS_DEST, &self.path, DEFAULT_TIMEOUT)
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
        secret: impl Into<SecretSlice<u8>>,
        salt: Vec<u8>,
    ) -> Result<Item<'_>> {
        let secret = secret.into().expose_secret().to_vec();
        let label = Box::new(service.to_string() + ":" + &account.to_string());
        let attrs: Box<HashMap<String, String>> = Box::new(HashMap::from_iter([
            (String::from("service"), service.to_string()),
            (String::from("account"), account.to_string()),
        ]));

        let mut props: PropMap = PropMap::new();
        props.insert(ITEM_LABEL.into(), Variant(label));
        props.insert(ITEM_ATTRIBUTES.into(), Variant(attrs));

        let session = self.service.session.path.clone();
        let secret = (session, salt, secret, "text/plain");
        let (item_path, prompt_path) = self
            .proxy()
            .create_item(props, secret, true)
            .map_err(Error::CreateItemError)?;

        let item_path = if item_path == Path::default() {
            // no creation path, so prompt
            self.service.prompt(&prompt_path)?
        } else {
            item_path
        };

        Ok(Item::new(&self.service, item_path))
    }

    pub fn delete_item(&self, service: impl AsRef<str>, account: impl AsRef<str>) -> Result<()> {
        self.get_item(service, account)?.delete()?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct Item<'a> {
    service: &'a SecretService,
    pub path: Path<'a>,
}

impl<'a> Item<'a> {
    pub fn new(service: &'a SecretService, path: Path<'a>) -> Self {
        Self { service, path }
    }

    pub fn proxy(&self) -> Proxy<'_, &'a Connection> {
        self.service
            .connection
            .with_proxy(DBUS_DEST, &self.path, DEFAULT_TIMEOUT)
    }

    pub fn get_secret(&self) -> Result<(Path<'static>, Vec<u8>, Vec<u8>, String)> {
        let proxy = &self.proxy();
        let session = self.service.session.path.clone();
        OrgFreedesktopSecretItem::get_secret(proxy, session).map_err(Error::GetSecretError)
    }

    pub fn delete(&self) -> Result<Path> {
        let proxy = &self.proxy();
        OrgFreedesktopSecretItem::delete(proxy).map_err(Error::DeleteItemError)
    }
}

#[derive(Debug)]
pub struct IoConnector {
    service: String,
    dbus: SecretService,
}

impl IoConnector {
    pub fn new(service: impl ToString, encryption: Algorithm) -> Result<Self> {
        Ok(Self {
            service: service.to_string(),
            dbus: SecretService::connect(encryption)?,
        })
    }

    pub fn session(&mut self) -> &mut Session {
        &mut self.dbus.session
    }

    #[cfg(feature = "secret-service-crypto")]
    pub fn read<F: GetKey + PutSecret + PutSalt>(&mut self, flow: &mut F) -> Result<()> {
        let (_, salt, secret, _) = self
            .dbus
            .get_default_collection()?
            .get_item(&self.service, flow.get_key())?
            .get_secret()?;
        flow.put_secret(secret.into());
        flow.put_salt(salt);
        Ok(())
    }

    #[cfg(not(feature = "secret-service-crypto"))]
    pub fn read<F: GetKey + PutSecret>(&mut self, flow: &mut F) -> Result<()> {
        let (_, _, secret, _) = self
            .dbus
            .get_default_collection()?
            .get_item(&self.service, flow.get_key())?
            .get_secret()?;
        flow.put_secret(secret.into());
        Ok(())
    }

    #[cfg(feature = "secret-service-crypto")]
    pub fn write<F: GetKey + TakeSecret + TakeSalt>(&mut self, flow: &mut F) -> Result<()> {
        let secret = flow.take_secret().ok_or(Error::WriteEmptySecretError)?;
        let salt = flow.take_salt().unwrap_or_default();

        self.dbus.get_default_collection()?.create_item(
            &self.service,
            flow.get_key(),
            secret,
            salt,
        )?;

        Ok(())
    }

    #[cfg(not(feature = "secret-service-crypto"))]
    pub fn write<F: GetKey + TakeSecret>(&mut self, flow: &mut F) -> Result<()> {
        let secret = flow.take_secret().ok_or(Error::WriteEmptySecretError)?;

        self.dbus.get_default_collection()?.create_item(
            &self.service,
            flow.get_key(),
            secret,
            vec![],
        )?;

        Ok(())
    }

    pub fn delete<F: GetKey>(&mut self, flow: &mut F) -> Result<()> {
        self.dbus
            .get_default_collection()?
            .get_item(&self.service, flow.get_key())?
            .delete()?;

        Ok(())
    }
}
