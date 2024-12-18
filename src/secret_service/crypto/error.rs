use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("cannot find public key for encrypted Secret Service communication")]
    FindPubkeyError,
    #[error("cannot find private key for encrypted Secret Service communication")]
    FindPrivkeyError,

    #[error("cannot encrypt undefined secret")]
    EncryptUndefinedSecretError,
    #[error("cannot encrypt secret: missing key")]
    EncryptSecretMissingKeyError,
    #[cfg(feature = "secret-service-openssl-std")]
    #[error("cannot encrypt secret using OpenSSL")]
    EncryptSecretOpensslError(#[source] openssl::error::ErrorStack),

    #[error("cannot decrypt undefined secret")]
    DecryptUndefinedSecretError,
    #[error("cannot decrypt secret: missing key")]
    DecryptSecretMissingKeyError,
    #[cfg(feature = "secret-service-openssl-std")]
    #[error("cannot decrypt secret using OpenSSL")]
    DecryptSecretOpensslError(#[source] openssl::error::ErrorStack),
    #[cfg(feature = "secret-service-rust-crypto-std")]
    #[error("cannot decrypt secret using Rust Crypto")]
    DecryptSecretRustCryptoError(#[source] block_padding::UnpadError),

    #[cfg(feature = "secret-service-openssl-std")]
    #[error("cannot derive shared key using OpenSSL")]
    DeriveSharedKeyOpensslError(#[source] openssl::error::ErrorStack),
    #[cfg(feature = "secret-service-rust-crypto-std")]
    #[error("cannot derive shared key using Rust Crypto")]
    DeriveSharedKeyRustCryptoError(#[source] hkdf::InvalidLength),
}

pub type Result<T> = std::result::Result<T, Error>;
