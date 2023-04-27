use crate::dicom_json::*;
use crate::ir_to_dimble::{HeaderField, HeaderFieldMap};
use memmap2::MmapOptions;
use rmpv::{decode, Value};
use std::fs;
use std::io::Cursor;

fn headerfield_and_bytes_to_dicom_fields(
    tag: &str,
    header_field: &HeaderField,
    dimble_buffer: &[u8],
) -> DicomField {
    match header_field {
        HeaderField::Empty(vr) => {
            let vr = String::from_utf8(vr.to_vec()).unwrap();
            DicomField {
                value: None,
                vr,
                inline_binary: None,
            }
        }
        HeaderField::SQ(sqs) => {
            let seq_fields = sqs
                .iter()
                .map(|sq| {
                    let mut sq_data: DicomJsonData = DicomJsonData::new();
                    for (tag, header_field) in sq.iter() {
                        let field: DicomField =
                            headerfield_and_bytes_to_dicom_fields(tag, header_field, dimble_buffer);
                        sq_data.insert(tag.to_string(), field);
                    }
                    DicomValue::SeqField(sq_data)
                })
                .collect::<Vec<_>>();
            // for sq in sqs {
            //     let mut sq_data: DicomJsonData = DicomJsonData::new();
            //     for (tag, header_field) in sq.iter() {
            //         let field: DicomField =
            //             headerfield_and_bytes_to_dicom_fields(tag, header_field, dimble_buffer);
            //         sq_data.insert(tag.to_string(), field);
            //     }
            //     seq_fields.push(DicomValue::SeqField(sq_data));
            // }
            // let mut sq_data: DicomJsonData = DicomJsonData::new();

            // let sq = sq.first().expect("should be at least one element");
            // for (tag, header_field) in sq.iter() {
            //     let field: DicomField =
            //         headerfield_and_bytes_to_dicom_fields(tag, header_field, dimble_buffer);
            //     sq_data.insert(tag.to_string(), field);
            // }

            DicomField {
                value: Some(seq_fields),
                vr: "SQ".to_string(),
                inline_binary: None,
            }
        }
        HeaderField::Deffered(field_pos, field_length, vr) => {
            let vr = String::from_utf8(vr.to_vec()).expect("expected vr to be utf8");
            // inline_binary VRs are OB and OW. TODO support the other inline binary VRs
            let field_pos: usize = (*field_pos as usize) + 8;
            let field_length = *field_length as usize;
            let field_bytes = &dimble_buffer[field_pos..field_pos + field_length];
            let dicom_field: DicomField = match vr.as_str() {
                "OB" | "OW" => {
                    let inline_binary: String = match tag {
                        "7FE00010" => {
                            // Pixel Data
                            "TODO encode pixel data correctly".to_string()
                        }
                        _ => {
                            let mut cursor = Cursor::new(field_bytes);
                            let v = decode::read_value(&mut cursor).unwrap();
                            v.as_str().unwrap().to_string()
                        }
                    };

                    DicomField {
                        value: None,
                        vr,
                        inline_binary: Some(inline_binary),
                    }
                }
                "PN" => {
                    let mut cursor = Cursor::new(field_bytes);
                    let v = decode::read_value(&mut cursor).unwrap();
                    let name = match v {
                        Value::String(s) => s.into_str().unwrap(),
                        _ => panic!("expected string"),
                    };
                    let a = DicomValue::Alphabetic(Alphabetic { alphabetic: name });
                    DicomField {
                        value: Some(vec![a]),
                        vr,
                        inline_binary: None,
                    }
                }
                _ => {
                    let mut cursor = Cursor::new(field_bytes);
                    let v = decode::read_value(&mut cursor).unwrap();
                    let value: Vec<DicomValue> = match v {
                        Value::String(s) => vec![DicomValue::String(s.into_str().unwrap())],
                        Value::Integer(i) => {
                            if i.is_i64() {
                                vec![DicomValue::Integer(i.as_i64().unwrap())]
                            } else {
                                vec![DicomValue::Integer(i.as_u64().unwrap() as i64)]
                            }
                        }
                        Value::F64(f) => vec![DicomValue::Float(f)],
                        Value::Array(a) => {
                            let mut values = Vec::new();
                            for v in a {
                                match v {
                                    Value::String(s) => {
                                        values.push(DicomValue::String(s.into_str().unwrap()))
                                    }
                                    Value::Integer(i) => {
                                        if i.is_i64() {
                                            values.push(DicomValue::Integer(i.as_i64().unwrap()))
                                        } else {
                                            values.push(DicomValue::Integer(
                                                i.as_u64().unwrap() as i64
                                            ))
                                        }
                                    }
                                    Value::F64(f) => values.push(DicomValue::Float(f)),
                                    _ => {
                                        println!("unexpected value type: {:?}", v);
                                        panic!("unexpected value type")
                                    }
                                };
                            }
                            values
                        }
                        _ => {
                            println!("unexpected value type: {:?}", v);
                            panic!("unexpected value type")
                        }
                    };
                    DicomField {
                        value: Some(value),
                        vr,
                        inline_binary: None,
                    }
                }
            };
            dicom_field
        }
    }
}

pub fn dimble_to_dicom_json(dimble_path: &str, json_path: &str) {
    let dimble_file = fs::File::open(dimble_path).unwrap();
    let dimble_buffer = unsafe { MmapOptions::new().map(&dimble_file).expect("mmap failed") };

    let (header, header_len) = deserialise_header(&dimble_buffer);

    let mut json_dicom = DicomJsonData::new();

    for (tag, header_field) in header.iter() {
        let field: DicomField =
            headerfield_and_bytes_to_dicom_fields(tag, header_field, &dimble_buffer[header_len..]);
        json_dicom.insert(tag.to_string(), field);
    }

    let json_file = fs::File::create(json_path).unwrap();
    serde_json::to_writer_pretty(json_file, &json_dicom).unwrap(); // TODO don't write pretty (this is for debugging)
}

// pub fn dimble_to_dicom_json_old(dimble_path: &str, json_path: &str) {
//     let dimble_file = fs::File::open(dimble_path).unwrap();
//     let dimble_buffer = unsafe { MmapOptions::new().map(&dimble_file).expect("mmap failed") };

//     let (header, header_len) = deserialise_header(&dimble_buffer);

//     let mut json_dicom = DicomJsonData::new();

//     for (tag, header_field) in header.iter() {
//         match header_field {
//             HeaderField::SQ(_sq) => {

//             }
//             HeaderField::Deffered(field_pos, field_length, vr) => {
//                 let vr = String::from_utf8(vr.to_vec()).expect("expected vr to be utf8");
//                 // inline_binary VRs are OB and OW. TODO support the other inline binary VRs
//                 let field_pos: usize = (*field_pos as usize) + header_len + 8;
//                 let field_length = *field_length as usize;
//                 let field_bytes = &dimble_buffer[field_pos..field_pos + field_length];
//                 let dicom_field: DicomField = match vr.as_str() {
//                     "OB" | "OW" => {
//                         let inline_binary: String = match tag.as_str() {
//                             "7FE00010" => {
//                                 // Pixel Data
//                                 "TODO encode pixel data correctly".to_string()
//                             }
//                             _ => {
//                                 let mut cursor = Cursor::new(field_bytes);
//                                 let v = decode::read_value(&mut cursor).unwrap();
//                                 v.as_str().unwrap().to_string()
//                             }
//                         };

//                         DicomField {
//                             value: None,
//                             vr,
//                             inline_binary: Some(inline_binary),
//                         }
//                     }
//                     "PN" => {
//                         let mut cursor = Cursor::new(field_bytes);
//                         let v = decode::read_value(&mut cursor).unwrap();
//                         let name = match v {
//                             Value::String(s) => s.into_str().unwrap(),
//                             _ => panic!("expected string"),
//                         };
//                         let a = DicomValue::Alphabetic(Alphabetic { alphabetic: name });
//                         DicomField {
//                             value: Some(vec![a]),
//                             vr,
//                             inline_binary: None,
//                         }
//                     }
//                     _ => {
//                         let mut cursor = Cursor::new(field_bytes);
//                         let v = decode::read_value(&mut cursor).unwrap();
//                         let value: Vec<DicomValue> = match v {
//                             Value::String(s) => vec![DicomValue::String(s.into_str().unwrap())],
//                             Value::Integer(i) => {
//                                 if i.is_i64() {
//                                     vec![DicomValue::Integer(i.as_i64().unwrap())]
//                                 } else {
//                                     vec![DicomValue::Integer(i.as_u64().unwrap() as i64)]
//                                 }
//                             }
//                             Value::F64(f) => vec![DicomValue::Float(f)],
//                             Value::Array(a) => {
//                                 let mut values = Vec::new();
//                                 for v in a {
//                                     match v {
//                                         Value::String(s) => {
//                                             values.push(DicomValue::String(s.into_str().unwrap()))
//                                         }
//                                         Value::Integer(i) => {
//                                             if i.is_i64() {
//                                                 values
//                                                     .push(DicomValue::Integer(i.as_i64().unwrap()))
//                                             } else {
//                                                 values.push(DicomValue::Integer(
//                                                     i.as_u64().unwrap() as i64,
//                                                 ))
//                                             }
//                                         }
//                                         Value::F64(f) => values.push(DicomValue::Float(f)),
//                                         _ => {
//                                             println!("unexpected value type: {:?}", v);
//                                             panic!("unexpected value type")
//                                         }
//                                     };
//                                 }
//                                 values
//                             }
//                             _ => {
//                                 println!("unexpected value type: {:?}", v);
//                                 panic!("unexpected value type")
//                             }
//                         };
//                         DicomField {
//                             value: Some(value),
//                             vr,
//                             inline_binary: None,
//                         }
//                     }
//                 };
//                 json_dicom.insert(tag.into(), dicom_field);
//             }
//             HeaderField::Empty(vr) => {
//                 let vr = String::from_utf8(vr.to_vec()).unwrap();
//                 let dicom_field = DicomField {
//                     value: None,
//                     vr,
//                     inline_binary: None,
//                 };
//                 json_dicom.insert(tag.into(), dicom_field);
//             }
//         }
//     }

//     let json_file = fs::File::create(json_path).unwrap();
//     serde_json::to_writer_pretty(json_file, &json_dicom).unwrap(); // TODO don't write pretty (this is for debugging)
// }

fn deserialise_header(buffer: &[u8]) -> (HeaderFieldMap, usize) {
    let header_len = u64::from_le_bytes(buffer[0..8].try_into().unwrap()) as usize;
    let header: HeaderFieldMap =
        rmp_serde::from_slice(&buffer[8..8 + header_len]).expect("failed to deserialise header");
    (header, header_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_deserialisation_single_string() {
        let buffer = {
            let mut header_fields = HeaderFieldMap::new();
            let vr = b"CS";
            header_fields.insert("00080005".to_string(), HeaderField::Deffered(0, 4, *vr));

            // serialise to buffer and prepend with header length
            let mut buffer = Vec::new();
            let header_bytes = rmp_serde::to_vec(&header_fields).unwrap();
            let header_len = header_bytes.len() as u64;
            buffer.extend_from_slice(&header_len.to_le_bytes());
            buffer.extend_from_slice(&header_bytes);
            buffer
        };

        let (header, _header_len) = deserialise_header(&buffer);
        println!("{:?}", header);
        if let HeaderField::Deffered(offset, length, vr) = *header.get("00080005").unwrap() {
            assert_eq!(offset, 0);
            assert_eq!(length, 4);
            assert_eq!(vr, *b"CS");
        } else {
            panic!("expected deffered header field");
        }
    }

    #[test]
    fn test_header_deserialisation_no_value() {
        let buffer = {
            let mut header_fields = HeaderFieldMap::new();
            let vr = b"PN";
            header_fields.insert("00100010".to_string(), HeaderField::Empty(*vr));

            // serialise to buffer and prepend with header length
            let mut buffer = Vec::new();
            let header_bytes = rmp_serde::to_vec(&header_fields).unwrap();
            let header_len = header_bytes.len() as u64;
            buffer.extend_from_slice(&header_len.to_le_bytes());
            buffer.extend_from_slice(&header_bytes);
            buffer
        };

        let (header, _header_len) = deserialise_header(&buffer);
        println!("{:?}", header);
    }
}
