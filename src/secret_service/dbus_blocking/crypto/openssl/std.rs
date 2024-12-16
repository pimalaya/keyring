use dbus::{
    arg::{cast, RefArg, Variant},
    blocking::Connection,
    Path,
};
use openssl::{
    cipher::Cipher, cipher_ctx::CipherCtx, error::ErrorStack, md::Md, pkey::Id, pkey_ctx::PkeyCtx,
};
use rand::{rngs::OsRng, Rng};
use secrecy::ExposeSecret;

use crate::secret_service::dbus_blocking::{
    self,
    api::OrgFreedesktopSecretService,
    crypto::{
        algorithm::Algorithm,
        common::{prepare_derive_shared, AesKey, Keypair},
        Error, Flow,
    },
    DBUS_DEST, DBUS_PATH, TIMEOUT,
};

pub struct IoConnector {
    pub encryption: Algorithm,
    pub session_path: Path<'static>,
    shared_key: Option<AesKey>,
}

impl IoConnector {
    pub fn new(
        connection: &Connection,
        encryption: Algorithm,
    ) -> Result<Self, dbus_blocking::std::Error> {
        let proxy = connection.with_proxy(DBUS_DEST, DBUS_PATH, TIMEOUT);
        let processor = match encryption {
            Algorithm::Plain => {
                let (_, session_path) = proxy
                    .open_session(encryption.as_ref(), Variant(Box::new(String::new())))
                    .map_err(dbus_blocking::std::Error::OpenSessionError)?;

                Self {
                    encryption,
                    session_path,
                    shared_key: None,
                }
            }
            Algorithm::Dh => {
                let keypair = Keypair::generate();

                // send our public key with algorithm to service
                let public_bytes = keypair.public.to_bytes_be();
                let bytes_arg = Variant(Box::new(public_bytes) as Box<dyn RefArg>);
                let (out, session_path) = proxy
                    .open_session(encryption.as_ref(), bytes_arg)
                    .map_err(dbus_blocking::std::Error::OpenSessionError)?;

                let Some(server_public_key_bytes) = cast::<Vec<u8>>(&out.0) else {
                    return Err(dbus_blocking::std::Error::CastServerPublicKeyToBytesError);
                };

                let shared_key = derive_shared(&keypair, server_public_key_bytes)
                    .map_err(Error::DeriveSharedKeyOpensslError)?;

                Self {
                    encryption,
                    session_path,
                    shared_key: Some(shared_key),
                }
            }
        };

        Ok(processor)
    }

    pub fn encrypt(&mut self, flow: &mut impl Flow) -> Result<(), Error> {
        let secret = flow
            .take_secret()
            .ok_or(Error::EncryptUndefinedSecretError)?;
        let secret = secret.expose_secret();
        let key = &self.shared_key.unwrap();

        let (secret, salt) = encrypt(secret, key).map_err(Error::EncryptSecretOpensslError)?;
        flow.give_secret(secret.into());
        flow.give_salt(salt);

        Ok(())
    }

    pub fn decrypt(&mut self, flow: &mut impl Flow) -> Result<(), Error> {
        let secret = flow
            .take_secret()
            .ok_or(Error::DecryptUndefinedSecretError)?;
        let secret = secret.expose_secret();
        let salt = flow.take_salt().unwrap_or_default();
        let key = &self.shared_key.unwrap();

        let secret = decrypt(secret, key, &salt).map_err(Error::DecryptSecretOpensslError)?;
        flow.give_secret(secret.into());

        Ok(())
    }
}

fn encrypt(data: &[u8], key: &AesKey) -> Result<(Vec<u8>, Vec<u8>), ErrorStack> {
    // create the salt for the encryption
    let mut aes_iv = [0u8; 16];
    OsRng.fill(&mut aes_iv);

    let mut ctx = CipherCtx::new()?;
    ctx.encrypt_init(Some(Cipher::aes_128_cbc()), Some(key), Some(&aes_iv))?;

    let mut output = vec![];
    ctx.cipher_update_vec(data, &mut output)?;
    ctx.cipher_final_vec(&mut output)?;

    Ok((output, aes_iv.to_vec()))
}

fn decrypt(encrypted_data: &[u8], key: &AesKey, iv: &[u8]) -> Result<Vec<u8>, ErrorStack> {
    let mut ctx = CipherCtx::new()?;
    ctx.decrypt_init(Some(Cipher::aes_128_cbc()), Some(key), Some(iv))?;

    let mut output = vec![];
    ctx.cipher_update_vec(encrypted_data, &mut output)?;
    ctx.cipher_final_vec(&mut output)?;
    Ok(output)
}

fn hkdf(ikm: Vec<u8>, salt: Option<&[u8]>, okm: &mut [u8]) -> Result<(), ErrorStack> {
    let mut ctx = PkeyCtx::new_id(Id::HKDF)?;
    ctx.derive_init()?;
    ctx.set_hkdf_md(Md::sha256())?;
    ctx.set_hkdf_key(&ikm)?;

    if let Some(salt) = salt {
        ctx.set_hkdf_salt(salt)?;
    }

    ctx.add_hkdf_info(&[]).unwrap();
    ctx.derive(Some(okm))?;

    Ok(())
}

fn derive_shared(keypair: &Keypair, server_public_key_bytes: &[u8]) -> Result<AesKey, ErrorStack> {
    let (ikm, mut okm) = prepare_derive_shared(keypair, server_public_key_bytes);
    hkdf(ikm, None, &mut okm)?;
    Ok(okm)
}
