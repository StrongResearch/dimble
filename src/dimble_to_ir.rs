use crate::dicom_json::*;
use crate::ir_to_dimble::{HeaderField, HeaderFieldMap};
use memmap2::MmapOptions;
use rmpv::{decode, Integer, Value};
use std::fs;

fn headerfield_and_bytes_to_dicom_fields(
    tag: &str,
    header_field: &HeaderField,
    dimble_buffer: &[u8],
) -> DicomField {
    match header_field {
        HeaderField::Empty(vr) => DicomField {
            value: None,
            vr: *vr,
            inline_binary: None,
        },
        HeaderField::SQ(sqs) => {
            let seq_fields = sqs
                .iter()
                .map(|sq| DicomValue::SeqField(headers_to_data(sq, dimble_buffer)))
                .collect::<Vec<_>>();

            DicomField {
                value: Some(seq_fields),
                vr: *b"SQ",
                inline_binary: None,
            }
        }
        HeaderField::Deffered(field_pos, field_length, vr) => {
            // inline_binary VRs are OB and OW. TODO support the other inline binary VRs
            let field_pos = (*field_pos as usize) + 8;
            let field_length = *field_length as usize;
            let field_bytes = &dimble_buffer[field_pos..field_pos + field_length];
            match vr {
                b"OB" | b"OW" => {
                    let inline_binary = match tag {
                        "7FE00010" => {
                            // Pixel Data
                            "TODO encode pixel data correctly".to_string()
                        }
                        _ => rmp_serde::decode::from_slice(field_bytes).unwrap(),
                    };

                    DicomField {
                        value: None,
                        vr: *vr,
                        inline_binary: Some(inline_binary),
                    }
                }
                b"PN" => {
                    let name = rmp_serde::decode::from_slice(field_bytes).unwrap();
                    let a = DicomValue::Alphabetic(Alphabetic { alphabetic: name });
                    DicomField {
                        value: Some(vec![a]),
                        vr: *vr,
                        inline_binary: None,
                    }
                }
                _ => {
                    let mut cursor = field_bytes;
                    let v = decode::read_value(&mut cursor).unwrap();
                    let value: Vec<_> = match v {
                        Value::String(s) => vec![DicomValue::String(s.into_str().unwrap())],
                        Value::Integer(i) => vec![integer_to_dicom_value(&i)],
                        Value::F64(f) => vec![DicomValue::Float(f)],
                        Value::Array(a) => a
                            .into_iter()
                            .map(|v| match v {
                                Value::String(s) => DicomValue::String(s.into_str().unwrap()),
                                Value::Integer(i) => integer_to_dicom_value(&i),
                                Value::F64(f) => DicomValue::Float(f),
                                _ => panic!("unexpected value type: {v:?}"),
                            })
                            .collect(),
                        _ => panic!("unexpected value type: {v:?}"),
                    };
                    DicomField {
                        value: Some(value),
                        vr: *vr,
                        inline_binary: None,
                    }
                }
            }
        }
    }
}

fn headers_to_data(sq: &HeaderFieldMap, dimble_buffer: &[u8]) -> DicomJsonData {
    sq.iter()
        .map(|(tag, header_field)| {
            let tag = tag.to_string();
            let field = headerfield_and_bytes_to_dicom_fields(&tag, header_field, dimble_buffer);
            (tag, field)
        })
        .collect()
}

fn integer_to_dicom_value(i: &Integer) -> DicomValue {
    if let Some(v) = i.as_i64() {
        DicomValue::Integer(v)
    } else if let Some(v) = i.as_u64() {
        DicomValue::Integer(v as i64)
    } else {
        panic!("Could not represent the integer as i64 or u64")
    }
}

pub fn dimble_to_dicom_json(dimble_path: &str, json_path: &str) {
    let dimble_file = fs::File::open(dimble_path).unwrap();
    let dimble_buffer = unsafe { MmapOptions::new().map(&dimble_file).expect("mmap failed") };

    let (header, header_len) = deserialise_header(&dimble_buffer);

    let json_dicom = headers_to_data(&header, &dimble_buffer[header_len..]);

    let json_file = fs::File::create(json_path).unwrap();
    serde_json::to_writer_pretty(json_file, &json_dicom).unwrap(); // TODO don't write pretty (this is for debugging)
}

fn deserialise_header(buffer: &[u8]) -> (HeaderFieldMap, usize) {
    let header_len = u64::from_le_bytes(buffer[0..8].try_into().unwrap()) as usize;
    let header =
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
