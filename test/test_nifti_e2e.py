import os
from pathlib import Path

import pytest

import dimble

PIXEL_ARRAY = "7FE00010"

TESTFILES_DIR = Path(__file__).parent.parent / "niivue-images"
assert TESTFILES_DIR.exists()

nifti_files = list(p for p in TESTFILES_DIR.glob("*.nii*") if "recon" not in p.name)

if os.getenv("E2ESMALL", None):
    nifti_files = nifti_files[:1]
assert len(nifti_files) > 0
nifti_files_ids = [p.name.split("?")[0] for p in nifti_files]


@pytest.mark.parametrize("nifti_file", nifti_files, ids=nifti_files_ids)
def test_nifti_to_dimble(nifti_file: Path, benchmark):
    dimble_file = Path("/tmp") / nifti_file.with_suffix(".dimble").name
    dimble.nifti_to_dimble(nifti_file, dimble_file)
    benchmark(dimble.nifti_to_dimble, nifti_file, dimble_file)


@pytest.mark.parametrize("nifti_file", nifti_files, ids=nifti_files_ids)
def test_load_dimble(nifti_file: Path, benchmark):
    dimble_file = Path("/tmp") / nifti_file.with_suffix(".dimble").name
    dimble.nifti_to_dimble(nifti_file, dimble_file)
    dimble.load_dimble(dimble_file, [PIXEL_ARRAY])
    benchmark(dimble.load_dimble, dimble_file, [PIXEL_ARRAY])


@pytest.mark.parametrize("nifti_file", nifti_files, ids=nifti_files_ids)
def test_dimble_to_nifti(nifti_file: Path, benchmark):
    dimble_file = Path("/tmp") / nifti_file.with_suffix(".dimble").name
    dimble.nifti_to_dimble(nifti_file, dimble_file)
    nifti_file = Path("/tmp") / dimble_file.with_suffix(".nii.gz").name
    dimble.dimble_to_nifti(dimble_file, nifti_file)
    benchmark(dimble.dimble_to_nifti, dimble_file, nifti_file)
