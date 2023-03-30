import json
from pathlib import Path

import numpy as np
import pydicom
from safetensors.numpy import save_file

from dimble_rs import dimble_rs


def _dicom_to_ir(dicom_path: Path, output_name: str) -> dict[str, Path]:
    ds = pydicom.dcmread(dicom_path)
    output_json = Path(output_name + ".json")
    output_pixel_array = Path(output_name + ".safetensors")
    with open(output_json, "w") as f:
        ds_json_dict = ds.to_json_dict()
        json.dump(ds_json_dict, f, sort_keys=True, indent=4)

    pixel_array = ds.pixel_array.astype(np.float32)
    save_file({"pixel_array": pixel_array}, output_pixel_array)
    return {"json": output_json, "pixel_array": output_pixel_array}


def _ir_to_dimble(json_path: Path, pixel_path: Path, output_path: Path) -> None:
    dimble_rs.dicom_json_to_dimble(str(json_path), str(output_path), str(pixel_path))


def dicom_to_dimble(dicom_path: Path, output_path: Path) -> None:
    dicom_path = Path(dicom_path)
    ir_paths = _dicom_to_ir(dicom_path, str(Path("/tmp") / (dicom_path.stem + ".ir")))
    try:
        _ir_to_dimble(ir_paths["json"], ir_paths["pixel_array"], output_path)
    finally:
        for path in ir_paths.values():
            path.unlink(missing_ok=True)


def load_dimble(path: Path, fields: list[str], device="cpu", slices=None):
    return dimble_rs.load_dimble(str(path), fields, device, slices)


def _dimble_to_ir(dimble_path: Path, output_path: Path) -> None:
    dimble_rs.dimble_to_dicom_json(str(dimble_path), str(output_path))


def dimble_to_dicom(dimble_path: Path, output_path: Path) -> None:
    dimble_path = Path(dimble_path)
    ir_path = Path("/tmp") / (dimble_path.stem + ".ir.json")
    try:
        _dimble_to_ir(dimble_path, ir_path)
        with open(ir_path) as f:
            ds = pydicom.Dataset.from_json(f.read())
        ds.is_little_endian = True
        ds.is_implicit_VR = True
        ds.save_as(output_path, write_like_original=False)
    finally:
        ir_path.unlink(missing_ok=True)
