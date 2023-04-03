from pathlib import Path
import pytest
import numpy as np
import dimble
import torch

TESTFILES_DIR = Path(__file__).parent.parent / "pydicom-data" / "data"
assert TESTFILES_DIR.exists()

TEST_DICOM_FILE = next(TESTFILES_DIR.iterdir())

DTYPES = [
    torch.uint8,
    torch.int8,
    torch.int16,
    torch.int32,
    torch.int64,
    torch.float16,
    torch.float32,
    torch.float64,
    torch.complex64,
    torch.complex128,
    "NOT A REAL DTYPE",
]

# same as above but with strings
DTYPES = [
    "uint8",
    "int8",
    "int16",
    "int32",
    "int64",
    "float16",
    "float32",
    "float64",
    # "complex64",
    # "complex128",
    "NOT A REAL DTYPE",
]

@pytest.mark.parametrize("dtype", DTYPES, ids=str)
def test_load_and_convert_dtype(dtype):
    dicom_file = TEST_DICOM_FILE
    dimble_file = Path("/tmp") / dicom_file.with_suffix(".dimble").name
    if dtype == "NOT A REAL DTYPE":
        with pytest.raises(Exception):
            dimble.dicom_to_dimble(dicom_file, dimble_file, dtype=dtype)
    else:
        dimble.dicom_to_dimble(dicom_file, dimble_file, dtype=dtype)
        ds = dimble.load_dimble(dimble_file, ["7FE00010"])
        assert str(ds["7FE00010"].dtype) == "torch." + dtype