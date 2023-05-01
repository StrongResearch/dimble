import json, tempfile
from pathlib import Path

import numpy as np
import pydicom
import SimpleITK as sitk
from dimble_rs import dimble_rs
from safetensors.numpy import save_file


def _create_temp_dir() -> Path:
    temp_dir = Path(tempfile.gettempdir())
    temp_dir.mkdir(parents=True, exist_ok=True)
    return temp_dir

def _dicom_to_ir(
    dicom_path: Path, output_name: str, dtype=np.float32
) -> dict[str, Path]:
    ds = pydicom.dcmread(dicom_path)
    output_json = Path(output_name + ".json")
    output_pixel_array = Path(output_name + ".safetensors")
    with open(output_json, "w") as f:
        ds_json_dict = ds.to_json_dict()
        json.dump(ds_json_dict, f, sort_keys=True, indent=4)

    pixel_array = ds.pixel_array.astype(dtype)
    save_file({"pixel_array": pixel_array}, output_pixel_array)
    return {"json": output_json, "pixel_array": output_pixel_array}


def _nifti_to_ir(
    image_path: Path, output_name: str, dtype=np.float32
) -> dict[str, Path]:
    # code adapted from https://stackoverflow.com/a/64012212
    itk_image = sitk.ReadImage(image_path)
    ds_json_dict = {}
    for k in itk_image.GetMetaDataKeys():
        entry = {
            "vr": "CS",
            "Value": [itk_image.GetMetaData(k)],
        }
        ds_json_dict[k] = entry
    ds_json_dict["7FE00010"] = {
        "vr": "OW",
        "Value": None,
        "InlineBinary": "Placeholder",
    }
    output_json = Path(output_name + ".json")
    output_pixel_array = Path(output_name + ".safetensors")
    with open(output_json, "w") as f:
        json.dump(ds_json_dict, f, sort_keys=True, indent=4)

    pixel_array = sitk.GetArrayFromImage(itk_image).astype(dtype)
    save_file({"pixel_array": pixel_array}, output_pixel_array)
    return {"json": output_json, "pixel_array": output_pixel_array}


def _ir_to_dimble(json_path: Path, pixel_path: Path, output_path: Path) -> None:
    dimble_rs.dicom_json_to_dimble(str(json_path), str(output_path), str(pixel_path))


def _dimble_to_ir(dimble_path: Path, output_path: Path) -> None:
    dimble_rs.dimble_to_dicom_json(str(dimble_path), str(output_path))


def dicom_to_dimble(dicom_path: Path, output_path: Path, dtype=np.float32) -> None:
    dicom_path = Path(dicom_path)
    ir_paths = _dicom_to_ir(
        dicom_path, str(_create_temp_dir() / (dicom_path.stem + ".ir")), dtype=dtype
    )
    try:
        _ir_to_dimble(ir_paths["json"], ir_paths["pixel_array"], output_path)
    finally:
        for path in ir_paths.values():
            path.unlink(missing_ok=True)


def nifti_to_dimble(image_path: Path, output_path: Path, dtype=np.float32) -> None:
    image_path = Path(image_path)
    ir_paths = _nifti_to_ir(
        image_path, str(_create_temp_dir() / (image_path.stem + ".ir")), dtype=dtype
    )
    try:
        _ir_to_dimble(ir_paths["json"], ir_paths["pixel_array"], output_path)
    finally:
        for path in ir_paths.values():
            path.unlink(missing_ok=True)


def load_dimble(path: Path, fields: list[str], device="cpu", slices=None):
    return dimble_rs.load_dimble(str(path), fields, device, slices)


def dimble_to_dicom(dimble_path: Path, output_path: Path) -> None:
    dimble_path = Path(dimble_path)
    ir_path = _create_temp_dir() / (dimble_path.stem + ".ir.json")
    dimble_ds = load_dimble(dimble_path, ["7FE00010"])

    pixel_data = dimble_ds["7FE00010"].numpy()
    from pydicom.uid import RLELossless

    try:
        _dimble_to_ir(dimble_path, ir_path)
        with open(ir_path) as f:
            ds: pydicom.Dataset = pydicom.Dataset.from_json(f.read())
        # TODO this code makes lots of unvalidated assumptions that should not be made.
        ds.BitsAllocated = 16
        ds.PixelRepresentation = 0
        ds.compress(RLELossless, pixel_data.astype(np.uint16))
        ds.save_as(output_path, write_like_original=False)
    finally:
        ir_path.unlink(missing_ok=True)


def dimble_to_nifti(dimble_path: Path, output_path: Path) -> None:
    dimble_path = Path(dimble_path)
    ir_path = _create_temp_dir() / (dimble_path.stem + ".ir.json")
    dimble_ds = load_dimble(dimble_path, ["7FE00010"])

    pixel_data = dimble_ds["7FE00010"].numpy()

    itk_image = sitk.GetImageFromArray(pixel_data)

    try:
        _dimble_to_ir(dimble_path, ir_path)
        with open(ir_path) as f:
            json_ds = json.load(f)
            for k in json_ds:
                if k != "7FE00010":
                    itk_image.SetMetaData(k, json_ds[k]["Value"][0])
        sitk.WriteImage(itk_image, output_path)
    finally:
        ir_path.unlink(missing_ok=True)
