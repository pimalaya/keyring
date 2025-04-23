//! Module dedicated to [`serde`] de/serialization of [`Entry`].

use std::fmt;

use serde::{
    de::{Error, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};

use crate::Entry;

impl Serialize for Entry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.name)
    }
}

impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Entry, D::Error> {
        deserializer.deserialize_string(EntryVisitor)
    }
}

struct EntryVisitor;

impl<'de> Visitor<'de> for EntryVisitor {
    type Value = Entry;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a keyring entry name (string)")
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
        Ok(Entry::new(v))
    }
}

#[cfg(test)]
mod tests {
    use serde::{
        de::value::{Error, StringDeserializer},
        Deserialize,
    };

    use crate::Entry;

    // TODO
    // #[test]
    // fn serialize() {
    //
    // }

    #[test]
    fn deserialize() {
        let expected = Entry::new("name");

        let s = String::from("name");
        let s = StringDeserializer::<Error>::new(s);
        let got = Entry::deserialize(s).unwrap();

        assert_eq!(expected, got);
    }
}
