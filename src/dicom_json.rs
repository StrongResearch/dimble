use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::ir_to_dimble::VR;

pub type DicomJsonData = HashMap<String, DicomField>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Alphabetic {
    #[serde(rename = "Alphabetic")]
    pub alphabetic: String, // TODO support Ideographic and Phonetic
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(untagged)]
pub enum DicomValue {
    Integer(i64),
    Float(f64),
    String(String),
    Alphabetic(Alphabetic),
    SeqField(DicomJsonData),
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct DicomField {
    #[serde(rename = "Value")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Vec<DicomValue>>,
    #[serde(with = "vr_serialization")]
    pub vr: VR,
    #[serde(rename = "InlineBinary")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_binary: Option<String>,
}

mod vr_serialization {
    use serde::{
        de::Error as _, ser::Error as _, Deserialize, Deserializer, Serialize, Serializer,
    };
    use std::borrow::Cow;

    use super::VR;

    pub fn serialize<S>(value: &VR, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = std::str::from_utf8(value).map_err(S::Error::custom)?;
        value.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<VR, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = <Cow<'de, str>>::deserialize(deserializer)?;
        value.as_bytes().try_into().map_err(D::Error::custom)
    }
}
