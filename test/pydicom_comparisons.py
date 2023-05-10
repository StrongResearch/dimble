import json
import tempfile
from pathlib import Path
import os

import numpy as np
import pydicom
import SimpleITK as sitk
import gdcm
import pytest
import dimble

PIXEL_ARRAY = "7FE00010"


# These files euther use DICOM features unsupported by dimble or are corrupt.
# Eventually these will be moved out of this list because either dimble will
# support them or we will have asserts that specific and helpful error messages
# are raised.
ignore_files = [
    "OT-PAL-8-face.dcm",
    "emri_small_jpeg_2k_lossless_too_short.dcm",
    "bad_sequence.dcm",
    # problems with reconstruction
    "SC_rgb_expb_32bit_2frame.dcm",
    "SC_rgb_expb_32bit.dcm",
    "US1_J2KI.dcm",
    "SC_rgb_32bit.dcm",
    "US1_J2KR.dcm",
    "SC_rgb_32bit_2frame.dcm",
    "explicit_VR-UN.dcm",
    "SC_ybr_full_uncompressed.dcm",
    "color3d_jpeg_baseline.dcm",
]
TESTFILES_DIR = Path(__file__).parent.parent / "pydicom-data" / "data"
assert TESTFILES_DIR.exists()

dicom_files = list(
    p
    for p in TESTFILES_DIR.glob("*.dcm*")
    if p.name not in ignore_files and "recon" not in p.name
)
if os.getenv("E2ESMALL", None):
    dicom_files = dicom_files[:1]
assert len(dicom_files) > 0
dicom_files_ids = [p.name.split("?")[0] for p in dicom_files]

DIMBLE_FILES = {}

def convert_to_dimble(dicom_file: Path):
    dimble_file = Path("/tmp") / dicom_file.with_suffix(".dimble").name
    if not dimble_file.exists():
        dimble.dicom_to_dimble(str(dicom_file), str(dimble_file))
    DIMBLE_FILES[dicom_file] = dimble_file

for dicom_file in dicom_files:
    convert_to_dimble(dicom_file)

print("dimble_files", DIMBLE_FILES)

def pydicom_to_numpy(dicom_file: Path):
    ds = pydicom.dcmread(dicom_file)
    pixel_array = ds.pixel_array
    return pixel_array.sum()

def sitk_to_numpy(dicom_file: Path):
    ds = sitk.ReadImage(str(dicom_file))
    pixel_array = sitk.GetArrayFromImage(ds)
    return pixel_array.sum()

def _get_gdcm_to_numpy_typemap():
    """Returns the GDCM Pixel Format to numpy array type mapping."""
    _gdcm_np = {gdcm.PixelFormat.UINT8  :np.uint8,
                gdcm.PixelFormat.INT8   :np.int8,
                #gdcm.PixelFormat.UINT12 :numpy.uint12,
                #gdcm.PixelFormat.INT12  :numpy.int12,
                gdcm.PixelFormat.UINT16 :np.uint16,
                gdcm.PixelFormat.INT16  :np.int16,
                gdcm.PixelFormat.UINT32 :np.uint32,
                gdcm.PixelFormat.INT32  :np.int32,
                #gdcm.PixelFormat.FLOAT16:numpy.float16,
                gdcm.PixelFormat.FLOAT32:np.float32,
                gdcm.PixelFormat.FLOAT64:np.float64 }
    return _gdcm_np

def _get_numpy_array_type(gdcm_pixel_format):
    """Returns a numpy array typecode given a GDCM Pixel Format."""
    return _get_gdcm_to_numpy_typemap()[gdcm_pixel_format]


def _gdcm_to_numpy(image):
    """Converts a GDCM image to a numpy array.
    """
    pf = image.GetPixelFormat()

    assert pf.GetScalarType() in _get_gdcm_to_numpy_typemap().keys(), \
           "Unsupported array type %s"%pf

    shape = image.GetDimension(0) * image.GetDimension(1), pf.GetSamplesPerPixel()
    if image.GetNumberOfDimensions() == 3:
      shape = shape[0] * image.GetDimension(2), shape[1]

    dtype = _get_numpy_array_type(pf.GetScalarType())
    gdcm_array = image.GetBuffer()
    result = np.fromstring(gdcm_array, dtype=dtype)
    result.shape = shape
    return result

def gdcm_to_numpy(dicom_file: Path):
    reader = gdcm.ImageReader()
    reader.SetFileName(str(dicom_file))
    reader.Read()
    image = reader.GetImage()
    gdcm_array = image.GetBuffer()
    return gdcm_array

    
def dimble_to_numpy(dicom_file: Path):
    dimble_file = DIMBLE_FILES[dicom_file]
    ds = dimble.load_dimble(str(dimble_file), fields=[PIXEL_ARRAY])
    pixel_array = ds[PIXEL_ARRAY]
    return pixel_array.sum()


@pytest.mark.parametrize("dicom_file", dicom_files, ids=dicom_files_ids)
def test_pydicom_read(dicom_file: Path, benchmark):
    benchmark(pydicom_to_numpy, dicom_file)

@pytest.mark.parametrize("dicom_file", dicom_files, ids=dicom_files_ids)
def test_sitk_read(dicom_file: Path, benchmark):
    benchmark(sitk_to_numpy, dicom_file)

@pytest.mark.parametrize("dicom_file", dicom_files, ids=dicom_files_ids)
def test_gdcm_read(dicom_file: Path, benchmark):
    benchmark(gdcm_to_numpy, dicom_file)

@pytest.mark.parametrize("dicom_file", dicom_files, ids=dicom_files_ids)
def test_dimble_read(dicom_file: Path, benchmark):
    benchmark(dimble_to_numpy, dicom_file)