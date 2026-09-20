//! UUID serde helpers. Swift writes uppercase hyphenated UUIDs; we accept any case and write uppercase.
pub mod upper {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use uuid::Uuid;

    pub fn serialize<S: Serializer>(id: &Uuid, s: S) -> Result<S::Ok, S::Error> {
        id.as_hyphenated().to_string().to_uppercase().serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Uuid, D::Error> {
        let text = String::deserialize(d)?;
        Uuid::parse_str(&text).map_err(serde::de::Error::custom)
    }
}

pub mod upper_opt {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use uuid::Uuid;

    pub fn serialize<S: Serializer>(id: &Option<Uuid>, s: S) -> Result<S::Ok, S::Error> {
        match id {
            Some(id) => id.as_hyphenated().to_string().to_uppercase().serialize(s),
            None => s.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Uuid>, D::Error> {
        let text: Option<String> = Option::deserialize(d)?;
        match text {
            Some(t) => Uuid::parse_str(&t).map(Some).map_err(serde::de::Error::custom),
            None => Ok(None),
        }
    }
}

pub fn upper_string(id: &uuid::Uuid) -> String {
    id.as_hyphenated().to_string().to_uppercase()
}
