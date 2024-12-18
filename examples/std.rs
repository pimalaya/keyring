use std::env;

#[cfg(feature = "secret-service")]
use keyring::secret_service::crypto::{Algorithm, Provider};
use keyring::std::Keyring;
use secrecy::ExposeSecret;

fn main() {
    let service = env::var("SERVICE").unwrap_or(String::from("test-service"));
    let key = env::var("KEY").unwrap_or(String::from("test-key"));
    let val = env::var("VAL").unwrap_or(String::from("test-val"));

    println!("using service name: {service:?}");
    println!("using entry key: {key:?}");
    println!("using entry value: {val:?}");

    #[cfg(feature = "secret-service")]
    let ss_crypto_algorithm = match env::var("SS_CRYPTO_ALGORITHM") {
        Ok(crypto) if crypto.trim().eq_ignore_ascii_case("plain") => Algorithm::Plain,
        Ok(crypto) if crypto.trim().eq_ignore_ascii_case("dh") => Algorithm::Dh,
        _ => Algorithm::Plain,
    };

    #[cfg(feature = "secret-service")]
    let ss_crypto_provider =
        match env::var("SS_CRYPTO_PROVIDER").expect("missing SS_CRYPTO_PROVIDER env var") {
            #[cfg(feature = "secret-service-openssl-std")]
            var if var.trim().eq_ignore_ascii_case("openssl") => {
                Provider::Openssl(ss_crypto_algorithm.clone())
            }
            #[cfg(feature = "secret-service-rust-crypto-std")]
            var if var.trim().eq_ignore_ascii_case("rust-crypto") => {
                Provider::RustCrypto(ss_crypto_algorithm.clone())
            }
            _ => panic!("cannot select std secret service crypto provider"),
        };

    let mut keyring = match env::var("KEYRING_PROVIDER").expect("missing KEYRING_PROVIDER env var")
    {
        #[cfg(feature = "apple-keychain-std")]
        var if var.trim().eq_ignore_ascii_case("apple-keychain") => {
            println!("using Apple Keychain");
            Keyring::apple_keychain(&service)
        }
        #[cfg(feature = "windows-credentials-std")]
        var if var.trim().eq_ignore_ascii_case("windows-credentials") => {
            println!("using Windows Credentials");
            Keyring::windows_credentials(&service)
        }
        #[cfg(feature = "secret-service-dbus-std")]
        var if var.trim().eq_ignore_ascii_case("dbus-secret-service") => {
            println!("using Secret Service with D-Bus");
            println!("using Secret Service crypto provider: {ss_crypto_provider:?}");
            Keyring::dbus_secret_service(&service, ss_crypto_provider).unwrap()
        }
        #[cfg(feature = "secret-service-zbus-std")]
        var if var.trim().eq_ignore_ascii_case("zbus-secret-service") => {
            println!("using Secret Service with Z-Bus");
            println!("using Secret Service crypto provider: {ss_crypto_provider:?}");
            Keyring::zbus_secret_service(&service, ss_crypto_provider).unwrap()
        }
        _ => panic!("cannot select std keyring provider"),
    };

    keyring.write(&key, val.as_bytes().to_vec()).unwrap();
    println!("write secret {val:?} to entry {service}:{key}");

    let secret = keyring.read(&key).unwrap();
    let secret = String::from_utf8_lossy(secret.expose_secret());
    println!("read secret {secret:?} from entry {service}:{key}");

    keyring.delete(&key).unwrap();
    println!("delete entry {service}:{key}");

    let err = keyring.read(&key).unwrap_err();
    println!("cannot read secret anymore: {err:?}");
}
