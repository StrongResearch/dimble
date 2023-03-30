import pytest
from pathlib import Path
import pydicom
import dimble

PIXEL_ARRAY = "7FE00010"

ignore_files = ["OT-PAL-8-face.dcm", "emri_small_jpeg_2k_lossless_too_short.dcm", "bad_sequence.dcm"]
TESTFILES_DIR = Path(__file__).parent.parent / "pydicom-data" / "data"
assert TESTFILES_DIR.exists()

dicom_files = list(p for p in TESTFILES_DIR.glob("*.dcm*") if p.name not in ignore_files)
assert len(dicom_files) > 0
dicom_files_ids = [p.name.split("?")[0] for p in dicom_files]

@pytest.mark.parametrize("dicom_file", dicom_files, ids=dicom_files_ids)
def test_dicom_to_dimble(dicom_file: Path):
    dimble_file = Path("/tmp") / dicom_file.with_suffix(".dimble")
    ds = pydicom.dcmread(dicom_file)
    dimble.dicom_to_dimble(dicom_file, dimble_file)
    print(dicom_file)

@pytest.mark.parametrize("dicom_file", dicom_files, ids=dicom_files_ids)
def test_load_dimble(dicom_file: Path):
    dimble_file = Path("/tmp") / dicom_file.with_suffix(".dimble")
    ds = pydicom.dcmread(dicom_file)
    dimble.dicom_to_dimble(dicom_file, dimble_file)
    dimble_ds = dimble.load_dimble(dimble_file, [PIXEL_ARRAY])


# @pytest.mark.parametrize("dicom_file", dicom_files, ids=dicom_files_ids)
# def test_dimble_to_dicom(dicom_file: Path):
#     dimble_file = Path("/tmp") / dicom_file.with_suffix(".dimble")
#     ds = pydicom.dcmread(dicom_file)
#     dimble.dicom_to_dimble(dicom_file, dimble_file)
#     reconstructed_dicom_file = Path("/tmp") / dicom_file.with_suffix(".recon.dcm")
#     dimble.dimble_to_dicom(dimble_file, reconstructed_dicom_file)