import json
from pathlib import Path

import pydicom

import dimble
from dimble.dimble import _dicom_to_ir

SQ_TAG = "00089215"

TESTFILES_DIR = Path(__file__).parent.parent / "pydicom-data" / "data"
assert TESTFILES_DIR.exists()

TEST_DICOM_FILE = TESTFILES_DIR / "693_J2KR.dcm"  # has a sequence


def test_sq_ir():
    """This is a fragile test because it tests a 'private' function. Mostly here for debugging"""
    output_name = "/tmp/sqir-693_J2KR.ir"
    ir_paths = _dicom_to_ir(TEST_DICOM_FILE, output_name)
    print(ir_paths)
    with open(ir_paths["json"], "r") as f:
        ir_data = json.load(f)
    element = ir_data[SQ_TAG]
    vr = element["vr"]
    assert vr == "SQ"
    sq = element["Value"]
    assert len(sq) == 1
    sq_value = sq[0]
    assert len(sq_value) == 3


def test_sq_dimble():
    dimble_file = "/tmp/693_J2KR.dimble"
    dimble.dicom_to_dimble(TEST_DICOM_FILE, dimble_file)
    ds = dimble.load_dimble(dimble_file, [SQ_TAG])
    assert ds == {
        "00089215": {
            "00080102": "DCM",
            "00080104": "Full fidelity image",
            "00080100": "121327",
        }
    }


def test_sq_recon():
    dimble_file = "/tmp/693_J2KR.dimble"
    dimble.dicom_to_dimble(TEST_DICOM_FILE, dimble_file)
    reconstructed_dicom_file = (
        Path("/tmp") / TEST_DICOM_FILE.with_suffix(".recon.dcm").name
    )
    dimble.dimble_to_dicom(dimble_file, reconstructed_dicom_file)
    ds = pydicom.dcmread(reconstructed_dicom_file)
    assert ds.to_json_dict()["00089215"] == {
        "vr": "SQ",
        "Value": [
            {
                "00080100": {"vr": "SH", "Value": ["121327"]},
                "00080102": {"vr": "SH", "Value": ["DCM"]},
                "00080104": {"vr": "LO", "Value": ["Full fidelity image"]},
            }
        ],
    }
