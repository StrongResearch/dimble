mod dicom_json;
mod dimble_to_ir;
mod ir_to_dimble;
use ir_to_dimble::{HeaderField, HeaderFieldMap};
use memmap2::MmapOptions;
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
use safetensors::tensor::Dtype;
use safetensors::tensor::TensorInfo;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Cursor;

static TORCH_MODULE: GILOnceCell<Py<PyModule>> = GILOnceCell::new();

#[derive(Debug)]
pub enum DimbleError {
    IoError(std::io::Error),
    MsgpackError(rmp_serde::decode::Error),
    MsgpackValueError(rmpv::decode::Error),
    JsonError(serde_json::Error),
}

impl From<std::io::Error> for DimbleError {
    fn from(err: std::io::Error) -> Self {
        DimbleError::IoError(err)
    }
}

impl From<rmp_serde::decode::Error> for DimbleError {
    fn from(err: rmp_serde::decode::Error) -> Self {
        DimbleError::MsgpackError(err)
    }
}

impl From<rmpv::decode::Error> for DimbleError {
    fn from(err: rmpv::decode::Error) -> Self {
        DimbleError::MsgpackValueError(err)
    }
}

impl std::fmt::Display for DimbleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for DimbleError {}

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

/// Helper struct used only for safetensors deserialization
#[derive(Debug, Serialize, Deserialize)]
struct HashMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "__metadata__")]
    metadata: Option<HashMap<String, String>>,
    #[serde(flatten)]
    tensors: HashMap<String, TensorInfo>,
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
    let header_len = u64::from_le_bytes(
        buffer[0..8]
            .try_into()
            .expect("safetensors object should have 8 byte header"),
    ) as usize;
    let metadata: HashMetadata =
        serde_json::from_slice(&buffer[8..8 + header_len]).expect("metadata should be valid json");
    let arr_info = metadata
        .tensors
        .get("pixel_array")
        .expect("pixel_array should be in metadata");
    assert_eq!(arr_info.dtype, Dtype::F32); // this is what dimble is hardcoded to do for now. TODO support other dtypes.

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
        let torch_float32 = torch.getattr(intern!(py, "float32"))?; // TODO unhardcode
        let kwargs = [(intern!(py, "dtype"), torch_uint8)].into_py_dict(py);
        let view_kwargs = [(intern!(py, "dtype"), torch_float32)].into_py_dict(py);
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
            if i.is_i64() {
                i.as_i64().unwrap().into_py(py)
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

#[pyfunction]
fn load_dimble(
    filename: &str,
    fields: Vec<&str>,
    device: &str,
    slices: Option<Vec<&PySlice>>,
) -> PyResult<PyObject> {
    let file = File::open(filename).unwrap();
    let buffer = unsafe { MmapOptions::new().map(&file).expect("mmap should work") };

    let header_len = u64::from_le_bytes(
        buffer[0..8]
            .try_into()
            .expect("file should have 8 byte header"),
    ) as usize;

    let header: HeaderFieldMap =
        rmp_serde::from_slice(&buffer[8..8 + header_len]).expect("header should be valid");

    Python::with_gil(|py| -> PyResult<PyObject> {
        let obj = PyDict::new(py);

        for field in fields {
            if let HeaderField::Deffered(field_pos, field_length, _vr) =
                *header.get(field).expect("expected field to exist")
            {
                let field_pos = (field_pos as usize) + header_len + 8;
                let field_length = field_length as usize;

                match field {
                    "7FE00010" => {
                        let tensor = load_pixel_array(
                            filename,
                            field_pos,
                            field_length,
                            device,
                            slices.clone(),
                        )?;
                        obj.set_item("7FE00010", tensor)
                            .expect("inserting should work");
                    }
                    _ => {
                        let field_bytes = &buffer[field_pos..field_pos + field_length];
                        let mut cursor = Cursor::new(field_bytes);
                        let field_value = read_value(&mut cursor).expect("msg");
                        let py_field = value_to_py(py, field_value);
                        obj.set_item(field, py_field)
                            .expect("inserting should work");
                    }
                }
            }
        }

        Ok(obj.to_object(py))
    })
}

#[pymodule]
fn dimble_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(dicom_json_to_dimble))?;
    m.add_wrapped(wrap_pyfunction!(dimble_to_dicom_json))?;
    m.add_wrapped(wrap_pyfunction!(load_dimble))?;
    m.add_wrapped(wrap_pyfunction!(load_pixel_array))?;
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
        println!("{}", recon_json);
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
