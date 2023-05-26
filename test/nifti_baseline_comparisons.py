import json
import os
import tempfile
from pathlib import Path

import gdcm
import numpy as np
import pydicom
import pytest
import SimpleITK as sitk
import nibabel as nib

import dimble

PIXEL_ARRAY = "7FE00010"

ignore_files = [
    'visiblehuman.nii.gz'
]
TESTFILES_DIR = Path(__file__).parent.parent / "niivue-images"
assert TESTFILES_DIR.exists()

nifti_files = list(p for p in TESTFILES_DIR.glob("*.nii*") if p.name not in ignore_files and "recon" not in p.name)

if os.getenv("E2ESMALL", None):
    nifti_files = nifti_files[:1]
assert len(nifti_files) > 0
nifti_files_ids = [p.name.split("?")[0] for p in nifti_files]

NIFTI_FILES = {}

def convert_to_dimble(nifti_file: Path):
    dimble_file = Path("/tmp") / nifti_file.with_suffix(".dimble").name
    if not dimble_file.exists():
        dimble.nifti_to_dimble(str(nifti_file), str(dimble_file))
    NIFTI_FILES[nifti_file] = dimble_file


for nifti_file in nifti_files:
    convert_to_dimble(nifti_file)

print("nifti_file", NIFTI_FILES)

def nibabel_to_numpy(nifti_file: Path):
    return nib.load(str(nifti_file)).get_fdata()

def dimble_to_numpy(dimble_file: Path):
    return dimble.load_dimble(dimble_file, [PIXEL_ARRAY])[PIXEL_ARRAY]

def sitk_to_numpy(nifti_file: Path):
    return sitk.GetArrayFromImage(sitk.ReadImage(str(nifti_file)))

@pytest.mark.parametrize("nifti_file", nifti_files, ids=nifti_files_ids)
def test_sitk_read(nifti_file: Path, benchmark):
    benchmark(sitk_to_numpy, str(nifti_file))

@pytest.mark.parametrize("nifti_file", nifti_files, ids=nifti_files_ids)
def test_nibabel_read(nifti_file: Path, benchmark):
    benchmark(nibabel_to_numpy, nifti_file)

@pytest.mark.parametrize("nifti_file", nifti_files, ids=nifti_files_ids)
def test_dimble_read(nifti_file: Path, benchmark):
    benchmark(dimble_to_numpy, NIFTI_FILES[nifti_file])