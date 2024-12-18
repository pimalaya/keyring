pub const ALGORITHM_PLAIN: &str = "plain";
pub const ALGORITHM_DH: &str = "dh-ietf1024-sha256-aes128-cbc-pkcs7";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum Algorithm {
    #[default]
    Plain,
    Dh,
}

impl AsRef<str> for Algorithm {
    fn as_ref(&self) -> &str {
        match self {
            Self::Plain => ALGORITHM_PLAIN,
            Self::Dh => ALGORITHM_DH,
        }
    }
}
