mod dicom_json;
mod dimble_to_ir;
mod ir_to_dimble;
use ir_to_dimble::{HeaderField, HeaderFieldMap};
use memmap2::MmapOptions;
use pyo3::exceptions::PyFileNotFoundError;
use pyo3::intern;
use pyo3::once_cell::GILOnceCell;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PySlice;
use pyo3::wrap_pyfunction;
use rmpv::decode::read_value;
use rmpv::Value;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;

static TORCH_MODULE: GILOnceCell<Py<PyModule>> = GILOnceCell::new();
#[pyfunction]
fn dicom_json_to_dimble(
    json_path: &str,
    dimble_path: &str,
    pixel_array_safetensors_path: Option<&str>,
) {
    ir_to_dimble::dicom_json_to_dimble(json_path, pixel_array_safetensors_path, dimble_path);
}

#[pyfunction]
fn dimble_to_dicom_json(dimble_path: &str, json_path: &str) {
    dimble_to_ir::dimble_to_dicom_json(dimble_path, json_path);
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TensorInfo {
    /// The type of each element of the tensor
    pub dtype: Dtype,
    /// The shape of the tensor
    pub shape: Vec<usize>,
    /// The offsets to find the data within the byte-buffer array.
    pub data_offsets: (usize, usize),
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[non_exhaustive]
pub enum Dtype {
    /// Boolan type
    BOOL,
    /// Unsigned byte
    U8,
    /// Signed byte
    I8,
    /// Signed integer (16-bit)
    I16,
    /// Unsigned integer (16-bit)
    U16,
    /// Half-precision floating point
    F16,
    /// Brain floating point
    BF16,
    /// Signed integer (32-bit)
    I32,
    /// Unsigned integer (32-bit)
    U32,
    /// Floating point (32-bit)
    F32,
    /// Floating point (64-bit)
    F64,
    /// Signed integer (64-bit)
    I64,
    /// Unsigned integer (64-bit)
    U64,
    /// Complex number (64-bit)
    C64,
    /// Complex number (128-bit)
    C128,
}

/// Helper struct used only for safetensors deserialization
#[derive(Debug, Serialize, Deserialize)]
struct HashMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "__metadata__")]
    metadata: Option<HashMap<String, String>>,
    #[serde(flatten)]
    tensors: HashMap<String, TensorInfo>,
}

fn get_pydtype(module: &PyModule, dtype: Dtype) -> PyResult<PyObject> {
    Python::with_gil(|py| {
        let dtype: PyObject = match dtype {
            Dtype::U8 => module.getattr(intern!(py, "uint8"))?.into(),
            Dtype::I8 => module.getattr(intern!(py, "int8"))?.into(),
            Dtype::I16 => module.getattr(intern!(py, "int16"))?.into(),
            Dtype::I32 => module.getattr(intern!(py, "int32"))?.into(),
            Dtype::I64 => module.getattr(intern!(py, "int64"))?.into(),
            Dtype::F16 => module.getattr(intern!(py, "float16"))?.into(),
            Dtype::F32 => module.getattr(intern!(py, "float32"))?.into(),
            Dtype::F64 => module.getattr(intern!(py, "float64"))?.into(),
            Dtype::BF16 => module.getattr(intern!(py, "bfloat16"))?.into(),
            Dtype::C64 => module.getattr(intern!(py, "complex64"))?.into(),
            Dtype::C128 => module.getattr(intern!(py, "complex128"))?.into(),
            dtype => {
                panic!("Dtype not understood: {dtype:?}");
            }
        };
        Ok(dtype)
    })
}

#[pyfunction]
pub fn load_pixel_array(
    filename: &str,
    st_offset: usize,
    st_length: usize,
    device: &str,
    slices: Option<Vec<&PySlice>>,
) -> PyResult<PyObject> {
    let file = File::open(filename).expect("file should exist");
    let buffer = unsafe {
        MmapOptions::new()
            .offset(st_offset as u64)
            .len(st_length)
            .map(&file)
            .expect("mmap should work")
    };
    // if buffer.len() < 8 {
    //     return DimbleError::new_err(format!("file too small to be a safetensors object"));
    // }
    let header_len = u64::from_le_bytes(
        buffer[0..8].try_into().map_err(|e| {
            DimbleError::new_err(format!(
                "safetensors object should have 8 byte header: {e:?}"
            ))
        })?,
        // .expect("safetensors object should have 8 byte header",
    ) as usize;
    let metadata: HashMetadata =
        serde_json::from_slice(&buffer[8..8 + header_len]).map_err(|e| {
            DimbleError::new_err(format!(
                "safetensors object should have valid json header: {e:?}"
            ))
        })?;
    let arr_info = metadata
        .tensors
        .get("pixel_array")
        .expect("pixel_array should be in metadata");

    let file_size = st_offset + st_length;
    let header_offset = header_len + 8;

    Python::with_gil(|py| -> PyResult<PyObject> {
        // setup
        let torch = TORCH_MODULE
            .get_or_init(py, || {
                PyModule::import(py, "torch")
                    .expect("Should be able to import torch")
                    .into()
            })
            .as_ref(py);
        let size = file_size.into_py(py);

        // make byte storage
        let py_filename: PyObject = filename.into_py(py);
        let shared = false.into_py(py);
        let storage_name = "UntypedStorage"; // TODO pt2.0 suppport
        let size_name = intern!(py, "nbytes");
        let kwargs = [(intern!(py, "shared"), shared), (size_name, size)].into_py_dict(py);
        let storage = torch
            .getattr(storage_name)?
            .getattr(intern!(py, "from_file"))?
            .call((py_filename,), Some(kwargs))?;

        // as array kwargs
        let torch_uint8 = torch.getattr(intern!(py, "uint8"))?;
        let torch_dtype = get_pydtype(torch, arr_info.dtype)?;
        let kwargs = [(intern!(py, "dtype"), torch_uint8)].into_py_dict(py);
        let view_kwargs = [(intern!(py, "dtype"), torch_dtype)].into_py_dict(py);
        let shape: PyObject = arr_info.shape.clone().into_py(py);

        // as array
        let start = st_offset + arr_info.data_offsets.0 + header_offset;
        let stop = st_offset + arr_info.data_offsets.1 + header_offset;
        let slice = PySlice::new(py, start as isize, stop as isize, 1);
        let storage_slice = storage
            .getattr(intern!(py, "__getitem__"))?
            .call1((slice,))?;
        let mut tensor = torch
            .getattr(intern!(py, "asarray"))?
            .call((storage_slice,), Some(kwargs))?
            .getattr(intern!(py, "view"))?
            .call((), Some(view_kwargs))?
            .getattr(intern!(py, "reshape"))?
            .call1((shape,))?;

        if let Some(slices) = slices {
            let slices = slices.into_py(py);
            tensor = tensor
                .getattr(intern!(py, "__getitem__"))?
                .call1((slices,))?;
        }

        if device != "cpu" {
            let device: PyObject = device.into_py(py);
            let kwargs = [(intern!(py, "device"), device)].into_py_dict(py);
            tensor = tensor.getattr(intern!(py, "to"))?.call((), Some(kwargs))?;
        }

        Ok(tensor.into_py(py))
    })
}

fn value_to_py(py: Python, value: Value) -> PyObject {
    match value {
        Value::String(s) => s.into_str().into_py(py),
        Value::F64(f) => f.into_py(py),
        Value::Integer(i) => {
            if let Some(v) = i.as_i64() {
                v.into_py(py)
            } else {
                i.as_u64().unwrap().into_py(py)
            }
        }
        Value::Array(a) => {
            let py_array = PyList::empty(py);
            for v in a {
                py_array.append(value_to_py(py, v)).unwrap();
            }
            py_array.into_py(py)
        }
        _ => panic!("unsupported value type"),
    }
}

fn get_field(py: Python, buffer: &[u8], field_pos: usize, field_length: usize) -> Py<PyAny> {
    let field_bytes = &buffer[field_pos..field_pos + field_length];
    let mut cursor = field_bytes;
    let field_value = read_value(&mut cursor).expect("should be valid messagepack"); // TODO better error handling
    value_to_py(py, field_value)
}

#[allow(clippy::too_many_arguments)]
fn header_fields_and_buffer_to_pydict(
    py: Python,
    header: &HeaderFieldMap,
    header_len: usize,
    dimble_buffer: &[u8],
    fields: Option<Vec<&str>>,
    filename: &str,
    device: &str,
    slices: &Option<Vec<&PySlice>>,
) -> PyResult<PyObject> {
    let dataset = PyDict::new(py);
    let fields = fields.unwrap_or_else(|| header.keys().map(|k| k.as_str()).collect());
    for field in fields {
        let py_field = match header.get(field) {
            Some(HeaderField::Deffered(field_pos, field_length, _vr)) => {
                // return the field value

                let field_pos = *field_pos as usize + header_len + 8;
                let field_length = *field_length as usize;

                match field {
                    "7FE00010" => {
                        load_pixel_array(filename, field_pos, field_length, device, slices.clone())?
                    }
                    _ => get_field(py, dimble_buffer, field_pos, field_length),
                }
            }
            Some(HeaderField::SQ(sq)) => {
                // return all fields of the sequence (In the future we might support lazy loading of sequence items)
                let sq = sq.first().expect("sq should have at least one item");
                header_fields_and_buffer_to_pydict(
                    py,
                    sq,
                    header_len,
                    dimble_buffer,
                    None,
                    filename,
                    device,
                    slices,
                )
                .unwrap()
            }
            Some(HeaderField::Empty(_vr)) => py.None(),
            None => panic!("field {field} not found for header {header:?}"),
        };

        dataset
            .set_item(field, py_field)
            .expect("inserting should work");
    }
    Ok(dataset.into_py(py))
}

fn deserialise_dimble_header(buffer: &[u8]) -> Result<(HeaderFieldMap, usize), DimbleError> {
    // TODO better error handling, this is a mess
    let header_len = u64::from_le_bytes(
        buffer[0..8]
            .try_into()
            .map_err(|e| {
                DimbleError::new_err(format!(
                    "safetensors object should have 8 byte header len: {e:?}"
                ))
            })
            .expect("file should have 8 byte header"),
    ) as usize;

    let header: HeaderFieldMap = rmp_serde::from_slice(&buffer[8..8 + header_len])
        .map_err(|e| {
            DimbleError::new_err(format!(
                "safetensors object should have valid header: {e:?}"
            ))
        })
        .expect("file should have valid header");

    Ok((header, header_len))
}

#[pyfunction]
fn load_dimble(
    filename: &str,
    fields: Vec<&str>,
    device: &str,
    slices: Option<Vec<&PySlice>>,
) -> PyResult<PyObject> {
    // this function takes in a filename and some fields and loads the data of those fields into a python dict

    let file = File::open(filename)
        .map_err(|_| PyFileNotFoundError::new_err(format!("file not found: {}", filename)))?;
    let buffer = unsafe { MmapOptions::new().map(&file).expect("mmap should work") };

    let (header, header_len) = deserialise_dimble_header(&buffer).expect("header should be valid"); // TODO better error handling

    let dataset = Python::with_gil(|py| {
        header_fields_and_buffer_to_pydict(
            py,
            &header,
            header_len,
            &buffer,
            Some(fields),
            filename,
            device,
            &slices,
        )
        .unwrap()
    });

    Ok(dataset)
}

pyo3::create_exception!(
    dimble_rs,
    DimbleError,
    pyo3::exceptions::PyException,
    "Custom Python Exception for Dimble errors."
);

#[pymodule]
fn dimble_rs(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(dicom_json_to_dimble))?;
    m.add_wrapped(wrap_pyfunction!(dimble_to_dicom_json))?;
    m.add_wrapped(wrap_pyfunction!(load_dimble))?;
    m.add_wrapped(wrap_pyfunction!(load_pixel_array))?;
    m.add("DimbleError", py.get_type::<DimbleError>())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_pixel_array_safetensors() {
        let path = "testfiles/eye3.safetensors";
        let size = std::fs::metadata(path).unwrap().len() as usize;

        pyo3::prepare_freethreaded_python();
        load_pixel_array(path, 0, size, "cpu", None).unwrap();
    }

    #[test]
    fn test_load_pixel_array_safetensors_cat2x() {
        let path = "testfiles/eye3.safetensors";
        let size = std::fs::metadata(path).unwrap().len() as usize;

        let path_cat3 = "testfiles/eye3.safetensors_cat3x";

        pyo3::prepare_freethreaded_python();
        load_pixel_array(path_cat3, 0, size, "cpu", None).unwrap();
        load_pixel_array(path_cat3, size, size, "cpu", None).unwrap();
    }

    #[test]
    fn test_load_pixel_array_safetensors_cat3x() {
        let path = "testfiles/eye3.safetensors";
        let size = std::fs::metadata(path).unwrap().len() as usize;

        let path_cat3 = "testfiles/eye3.safetensors_cat3x";

        pyo3::prepare_freethreaded_python();
        load_pixel_array(path_cat3, 0, size, "cpu", None).unwrap();
        load_pixel_array(path_cat3, size, size, "cpu", None).unwrap();
        load_pixel_array(path_cat3, 2 * size, size, "cpu", None).unwrap();
    }

    #[test]
    fn test_integration_single_string() {
        let dicom_json_text = r#"
        {
            "00080005": {
                "vr": "CS",
                "Value": [
                    "ISO_IR 100"
                ]
            } 
        }
        "#;
        let ir_path = "/tmp/single_string.ir.json";
        let dimble_path = "/tmp/single_string.dimble";
        let ir_recon_path = "/tmp/single_string.ir.recon.json";

        fs::write(ir_path, dicom_json_text).expect("should be able to write to file");

        dicom_json_to_dimble(ir_path, dimble_path, None);

        dimble_to_dicom_json(dimble_path, ir_recon_path);

        let recon_json_reader = fs::File::open(ir_recon_path).expect("should be able to open file");
        use serde_json::Value;
        let recon_json: Value =
            serde_json::from_reader(recon_json_reader).expect("should be able to read json");
        assert_eq!(recon_json["00080005"]["Value"][0], "ISO_IR 100");
        assert_eq!(recon_json["00080005"]["vr"], "CS");
    }

    #[test]
    fn test_integration_string_array() {
        let dicom_json_text = r#"
        {
            "00080008": {
                "vr": "CS",
                "Value": [
                    "ORIGINAL",
                    "PRIMARY",
                    "OTHER"
                ]
            } 
        }
        "#;
        let ir_path = "/tmp/single_string_array.ir.json";
        let dimble_path = "/tmp/single_string_array.dimble";
        let ir_recon_path = "/tmp/single_string_array.ir.recon.json";

        fs::write(ir_path, dicom_json_text).expect("should be able to write to file");

        dicom_json_to_dimble(ir_path, dimble_path, None);

        dimble_to_dicom_json(dimble_path, ir_recon_path);

        let recon_json_reader = fs::File::open(ir_recon_path).expect("should be able to open file");
        use serde_json::Value;
        let recon_json: Value =
            serde_json::from_reader(recon_json_reader).expect("should be able to read json");
        assert_eq!(recon_json["00080008"]["Value"][0], "ORIGINAL");
        assert_eq!(recon_json["00080008"]["Value"][1], "PRIMARY");
        assert_eq!(recon_json["00080008"]["Value"][2], "OTHER");
        assert_eq!(recon_json["00080008"]["vr"], "CS");
    }

    #[test]
    fn test_integration_no_value() {
        let dicom_json_text = r#"
        {
            "00080008": {
                "vr": "PN"
            } 
        }
        "#;
        let ir_path = "/tmp/no_value.ir.json";
        let dimble_path = "/tmp/no_value.dimble";
        let ir_recon_path = "/tmp/no_value.ir.recon.json";

        fs::write(ir_path, dicom_json_text).expect("should be able to write to file");

        dicom_json_to_dimble(ir_path, dimble_path, None);

        dimble_to_dicom_json(dimble_path, ir_recon_path);

        let recon_json_reader = fs::File::open(ir_recon_path).expect("should be able to open file");
        use serde_json::Value;
        let recon_json: Value =
            serde_json::from_reader(recon_json_reader).expect("should be able to read json");
        assert_eq!(recon_json["00080008"]["vr"], "PN");
        assert_eq!(recon_json["00080008"]["Value"], Value::Null);
    }

    #[test]
    fn test_integration_inline_binary() {
        let dicom_json_text = r#"
        {
            "00080008": {
                "vr": "OB",
                "InlineBinary": "ABCD"
            } 
        }
        "#;
        let ir_path = "/tmp/inline_binary.ir.json";
        let dimble_path = "/tmp/inline_binary.dimble";
        let ir_recon_path = "/tmp/inline_binary.ir.recon.json";

        fs::write(ir_path, dicom_json_text).expect("should be able to write to file");

        dicom_json_to_dimble(ir_path, dimble_path, None);

        dimble_to_dicom_json(dimble_path, ir_recon_path);

        let recon_json_reader = fs::File::open(ir_recon_path).expect("should be able to open file");
        use serde_json::Value;
        let recon_json: Value =
            serde_json::from_reader(recon_json_reader).expect("should be able to read json");
        assert_eq!(recon_json["00080008"]["vr"], "OB");
        assert_eq!(recon_json["00080008"]["InlineBinary"], "ABCD");
        // assert that there is no Value field
        assert!(
            !recon_json["00080008"]
                .as_object()
                .unwrap()
                .contains_key("Value"),
            "should not have Value field"
        );
    }
}
