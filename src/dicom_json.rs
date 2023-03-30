use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub vr: String,
    #[serde(rename = "InlineBinary")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_binary: Option<String>,
}
