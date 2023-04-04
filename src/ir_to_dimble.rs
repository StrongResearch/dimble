use rmp_serde::{to_vec, Serializer};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{BufReader, Write};

use crate::dicom_json::*;
use std::io::prelude::*;
use std::io::SeekFrom;

type VR = [u8; 2]; // TODO use newtype pattern?

#[derive(Debug, Serialize, Deserialize)]
pub enum HeaderField {
    // offset, length, VR
    Deffered(u64, u64, VR), // TODO use struct with names?
    Empty(VR),
    SQ(Vec<HeaderFieldMap>),
}

fn extend_and_make_field(data_bytes: &mut Vec<u8>, field_bytes: &[u8], vr: VR) -> HeaderField {
    let offset = data_bytes.len() as u64;
    data_bytes.extend_from_slice(field_bytes);
    HeaderField::Deffered(offset, field_bytes.len() as u64, vr)
}

pub type HeaderFieldMap = HashMap<String, HeaderField>;

// enum FieldBytes {
//     Bytes(Vec<u8>), // TODO ensure this is zero copy
//     Missing,
//     NotSupported,
// }

// fn get_field_bytes(
//     tag: &str,
//     dicom_field: &DicomField,
//     pixel_array_safetensors_path: Option<&str>,
// ) -> FieldBytes {
//     let field_bytes = match dicom_field {
//         DicomField {
//             value: Some(value),
//             vr,
//             inline_binary: None,
//         } =>
//         //"normal case -> PACK",
//         {
//             match value.as_slice() {
//                 [DicomValue::String(s)] => to_vec(&s).unwrap(),
//                 [DicomValue::Integer(u)] => to_vec(&u).unwrap(),
//                 [DicomValue::Float(u)] => to_vec(&u).unwrap(),
//                 [DicomValue::Alphabetic(u)] => to_vec(&u.alphabetic).unwrap(),
//                 [DicomValue::SeqField(_seq)] => {
//                     return FieldBytes::NotSupported;
//                 }
//                 [] if vr == "SQ" => return FieldBytes::NotSupported,
//                 many => match many
//                     .first()
//                     .expect("This should definitely have a first element")
//                 {
//                     DicomValue::String(_) => to_vec(
//                         &many
//                             .iter()
//                             .map(|v| match v {
//                                 DicomValue::String(s) => s.to_owned(),
//                                 _ => panic!("{tag} expected only strings"),
//                             })
//                             .collect::<Vec<String>>(),
//                     )
//                     .unwrap(),
//                     DicomValue::Integer(_) => to_vec(
//                         &many
//                             .iter()
//                             .map(|v| match v {
//                                 DicomValue::Integer(i) => *i,
//                                 _ => panic!("{tag} expected only strings"),
//                             })
//                             .collect::<Vec<i64>>(),
//                     )
//                     .unwrap(),
//                     DicomValue::Float(_) => to_vec(
//                         &many
//                             .iter()
//                             .map(|v| match v {
//                                 DicomValue::Float(f) => *f,
//                                 _ => panic!("{tag} expected only strings"),
//                             })
//                             .collect::<Vec<f64>>(),
//                     )
//                     .unwrap(),
//                     DicomValue::SeqField(_) => {
//                         return FieldBytes::NotSupported;
//                     }
//                     other => panic!("{tag} unexpected value type {:?}", other),
//                 },
//             }
//         }
//         DicomField {
//             value: None,
//             vr: _string,
//             inline_binary: None,
//         } =>
//         // "strange empty value case -> mark as missing",
//         {
//             return FieldBytes::Missing;
//         }
//         DicomField {
//             value: None,
//             vr: _string,
//             inline_binary: Some(inline_binary),
//         } => match tag {
//             "7FE00010" => get_file_bytes(
//                 pixel_array_safetensors_path.expect("expected pixel_array_safetensors_path"),
//             ),
//             _ => to_vec(&inline_binary).unwrap(),
//         },
//         DicomField {
//             value: Some(_),
//             vr: _string,
//             inline_binary: Some(_),
//         } => panic!("value and inline binary both present"),
//     };
//     FieldBytes::Bytes(field_bytes)
// }

fn get_file_bytes(safetensors_path: &str) -> Vec<u8> {
    let mut f = fs::File::open(safetensors_path).unwrap();
    let mut buffer = Vec::new();
    f.read_to_end(&mut buffer).unwrap();
    buffer
}

fn dicom_values_to_vec(tag: &str, dicom_values: &[DicomValue]) -> Option<Vec<u8>> {
    let field_bytes = match dicom_values {
        [DicomValue::String(s)] => to_vec(&s).unwrap(),
        [DicomValue::Integer(u)] => to_vec(&u).unwrap(),
        [DicomValue::Float(u)] => to_vec(&u).unwrap(),
        [DicomValue::Alphabetic(u)] => to_vec(&u.alphabetic).unwrap(),
        many => match many
            .first()
            .expect("This should definitely have a first element")
        {
            DicomValue::String(_) => to_vec(
                &many
                    .iter()
                    .map(|v| match v {
                        DicomValue::String(s) => s.to_owned(),
                        _ => panic!("{tag} expected only strings"),
                    })
                    .collect::<Vec<String>>(),
            )
            .unwrap(),
            DicomValue::Integer(_) => to_vec(
                &many
                    .iter()
                    .map(|v| match v {
                        DicomValue::Integer(i) => *i,
                        _ => panic!("{tag} expected only ints"),
                    })
                    .collect::<Vec<i64>>(),
            )
            .unwrap(),
            DicomValue::Float(_) => to_vec(
                &many
                    .iter()
                    .map(|v| match v {
                        DicomValue::Float(f) => *f,
                        _ => panic!("{tag} expected only floats"),
                    })
                    .collect::<Vec<f64>>(),
            )
            .unwrap(),
            DicomValue::SeqField(_) => {
                // TODO: handle sequences of sequences properly
                return None;
            }
            other => panic!("{tag} unexpected value type {:?}", other),
        },
    };
    Some(field_bytes)
}

fn prepare_dimble_fields(
    dicom_fields: &DicomJsonData,
    data_bytes: &mut Vec<u8>,
    pixel_array_safetensors_path: Option<&str>,
) -> HeaderFieldMap {
    let mut header_field_map: HeaderFieldMap = HeaderFieldMap::new();
    for (tag, dicom_field) in dicom_fields.iter() {
        let header_field: HeaderField =
            prepare_dimble_field(tag, dicom_field, data_bytes, pixel_array_safetensors_path);
        header_field_map.insert(tag.to_owned(), header_field);
    }
    header_field_map
}

fn prepare_dimble_field(
    tag: &str,
    dicom_field: &DicomField,
    data_bytes: &mut Vec<u8>,
    pixel_array_safetensors_path: Option<&str>,
) -> HeaderField {
    match dicom_field {
        DicomField {
            value: Some(value),
            vr,
            inline_binary: None,
        } => {
            match value.as_slice() {
                [] if vr == "SQ" => HeaderField::SQ(vec![]),
                [] => panic!("empty value"),
                [DicomValue::SeqField(seq)] => {
                    let sq_header_field_map =
                        prepare_dimble_fields(seq, data_bytes, pixel_array_safetensors_path);
                    HeaderField::SQ(vec![sq_header_field_map])
                }
                dicom_values => {
                    // call a function to handle this
                    match dicom_values_to_vec(tag, dicom_values) {
                        Some(field_bytes) => {
                            let vr = vr.as_bytes().try_into().unwrap();
                            extend_and_make_field(data_bytes, &field_bytes, vr)
                        }
                        None => {
                            // TODO this is kind of a hack for gracefully not handling sequences of sequences
                            HeaderField::Empty(vr.as_bytes().try_into().unwrap())
                        }
                    }
                }
            }
        }
        DicomField {
            value: None,
            vr,
            inline_binary: None,
        } => HeaderField::Empty(vr.as_bytes().try_into().unwrap()),
        DicomField {
            value: None,
            vr,
            inline_binary: Some(inline_binary),
        } => match tag {
            "7FE00010" => {
                let field_bytes = get_file_bytes(
                    pixel_array_safetensors_path.expect("expected pixel_array_safetensors_path"),
                );
                // data_bytes.extend(field_bytes);
                let vr = vr.as_bytes().try_into().unwrap();
                extend_and_make_field(data_bytes, &field_bytes, vr)
            }
            _ => {
                let field_bytes = to_vec(&inline_binary).unwrap();
                let vr = vr.as_bytes().try_into().unwrap();
                extend_and_make_field(data_bytes, &field_bytes, vr)
            }
        },
        DicomField {
            value: Some(_),
            vr: _vr,
            inline_binary: Some(_),
        } => panic!("value and inline binary both present"),
    }
}

fn prepare_dicom_fields_for_serialisation(
    dicom_json_data: DicomJsonData,
    pixel_array_safetensors_path: Option<&str>,
) -> (HeaderFieldMap, Vec<u8>) {
    let mut data_bytes: Vec<u8> = Vec::new();

    let header_fields = prepare_dimble_fields(
        &dicom_json_data,
        &mut data_bytes,
        pixel_array_safetensors_path,
    );

    (header_fields, data_bytes)
}

// fn prepare_dicom_fields_for_serialisation(
//     dicom_fields: DicomJsonData,
//     pixel_array_safetensors_path: Option<&str>,
// ) -> (HeaderFieldMap, Vec<u8>) {
//     let mut header_fields: HeaderFieldMap = HeaderFieldMap::new();
//     let mut data_bytes: Vec<u8> = Vec::new();
//     let mut offset: u64 = 0;

//     for (tag, dicom_field) in dicom_fields {
//         let vr: VR = dicom_field.vr.as_bytes().try_into().unwrap();
//         match get_field_bytes(&tag, &dicom_field, pixel_array_safetensors_path) {
//             FieldBytes::Bytes(bytes) => {
//                 let field_length = bytes.len() as u64;
//                 data_bytes.extend(bytes);
//                 header_fields.insert(tag, HeaderField::Deffered(offset, field_length, vr));
//                 offset += field_length;
//             }
//             FieldBytes::Missing => {
//                 header_fields.insert(tag, HeaderField::Empty(vr));
//             }
//             FieldBytes::NotSupported => {
//                 // TODO emit warning
//             }
//         }
//     }

//     (header_fields, data_bytes)
// }

fn serialise_dimble_fields(header_fields: HeaderFieldMap, data_bytes: Vec<u8>, dimble_path: &str) {
    let mut file = fs::File::create(dimble_path).unwrap();
    let mut file_copy = file.try_clone().unwrap(); //  copy this because rust complains about borrowing file twice
    file.seek(SeekFrom::Start(8)).unwrap(); // move 8 bytes forward to make room for header length (u64)
    let mut serialiser = Serializer::new(file).with_struct_map();

    header_fields.serialize(&mut serialiser).unwrap();
    let header_len: u64 = file_copy.stream_position().unwrap() - 8; // (u64) TODO maybe make a less magical number
    file_copy.seek(SeekFrom::Start(0)).unwrap();
    file_copy.write_all(&header_len.to_le_bytes()).unwrap();
    file_copy.seek(SeekFrom::Start(8 + header_len)).unwrap();

    let mut inner = serialiser.into_inner();
    inner.write_all(&data_bytes).unwrap();
}

pub fn dicom_json_to_dimble(
    json_path: &str,
    pixel_array_safetensors_path: Option<&str>,
    dimble_path: &str,
) {
    let json_reader = BufReader::new(fs::File::open(json_path).expect("json_path should exist"));
    let json_dicom = deserialise_ir(json_reader);

    let (header_fields, data_bytes) =
        prepare_dicom_fields_for_serialisation(json_dicom, pixel_array_safetensors_path);

    serialise_dimble_fields(header_fields, data_bytes, dimble_path);
}

fn deserialise_ir(data: impl Read) -> DicomJsonData {
    let dicom_json: HashMap<String, DicomField> =
        serde_json::from_reader(data).expect("Failed to parse JSON");
    dicom_json
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_ir_deserialisation() {
        let ir_data = r#"
        {
            "00080005": {
                "vr": "CS",
                "Value": [
                    "ISO_IR 100"
                ]
            },
            "00080008": {
                "vr": "CS",
                "Value": [
                    "ORIGINAL",
                    "PRIMARY",
                    "OTHER"
                ]
            },
            "00080090": {
                "vr": "PN"
            },
            "00100010": {
                "vr": "PN",
                "Value": [
                    {
                        "Alphabetic": "Doe^John"
                    }
                ]
            }
        }
        "#;

        let ir = deserialise_ir(ir_data.as_bytes());
        {
            let field = ir.get("00080005").expect("expected 00080005 to exist");
            assert_eq!(field.vr, "CS");
            let value: Vec<String> = field
                .value
                .iter()
                .map(|v| match v.as_slice() {
                    [DicomValue::String(s)] => s.to_owned(),
                    _ => panic!("expected only strings"),
                })
                .collect();
            assert_eq!(value, vec!["ISO_IR 100".to_owned()])
        }
        {
            let field = ir.get("00080008").expect("expected 00080008 to exist");
            assert_eq!(field.vr, "CS");
            let value: Vec<String> = field
                .value
                .as_ref()
                .unwrap()
                .iter()
                .map(|v| match v {
                    DicomValue::String(s) => s.to_owned(),
                    _ => panic!("expected only strings"),
                })
                .collect();
            assert_eq!(
                value,
                vec![
                    "ORIGINAL".to_owned(),
                    "PRIMARY".to_owned(),
                    "OTHER".to_owned()
                ]
            )
        }
        {
            let field = ir.get("00080090").expect("expected 00080090 to exist");
            assert_eq!(field.vr, "PN");
            assert_eq!(field.value, None);
        }
        {
            let field = ir.get("00100010").expect("expected 00100010 to exist");
            assert_eq!(field.vr, "PN");
            let value: Vec<String> = field
                .value
                .as_ref()
                .unwrap()
                .iter()
                .map(|v| match v {
                    DicomValue::Alphabetic(a) => a.alphabetic.to_owned(),
                    _ => panic!("expected only alphabetic"),
                })
                .collect();
            assert_eq!(value, vec!["Doe^John".to_owned()])
        }
    }

    #[test]
    fn test_serialise_dimble_fields() {
        let mut header_fields = HeaderFieldMap::new();
        let vr = b"CS";
        header_fields.insert("0008005".to_string(), HeaderField::Deffered(0, 1, *vr));
        let data_bytes = vec![0x42];
        let dimble_path = "/tmp/test.dimble";
        serialise_dimble_fields(header_fields, data_bytes, dimble_path);

        let file_bytes = fs::read(dimble_path).unwrap();
        assert_eq!(file_bytes.last().unwrap(), &0x42);
        let header_len = u64::from_le_bytes(file_bytes[0..8].try_into().unwrap()) as usize;
        let mut cursor = Cursor::new(&file_bytes[8..8 + header_len]);

        let _decoded = rmpv::decode::read_value(&mut cursor).unwrap();
    }

    // #[test]
    // fn test_get_field_bytes() {
    //     let dicom_field = DicomField {
    //         vr: "CS".to_string(),
    //         value: Some(vec![DicomValue::String("ORIGINAL".to_string())]),
    //         inline_binary: None,
    //     };

    //     let field_bytes = get_field_bytes("00001111", &dicom_field, None);

    //     assert!(matches!(field_bytes, FieldBytes::Bytes(_)));
    // }
}
