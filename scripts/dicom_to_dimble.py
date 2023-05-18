from pathlib import Path
from multiprocessing import Pool

import plac
from tqdm import tqdm

import dimble

def convert(dicom_dir: Path, dicom_files: list[Path], dimble_dir: Path):
    for dicom_file in tqdm(dicom_files):
        dimble_file = dimble_dir / dicom_file.relative_to(
            dicom_dir
        ).with_suffix(".dimble")
        if dimble_file.exists():
            continue
        dimble_file.parent.mkdir(parents=True, exist_ok=True)
        dimble.dicom_to_dimble(dicom_file, dimble_file)

@plac.annotations(
    dicom_path=("Path to DICOM directory", "positional", None, Path),
    dimble_path=("Path to DIMBLE directory", "positional", None, Path),
)
def main(dicom_path: Path, dimble_path: Path):
    dicom_files = list(dicom_path.glob("**/*.dcm")) + list(dicom_path.glob("**/*.dicom")) + list(dicom_path.glob("**/*.DCM"))
    # convert(dicom_path, dicom_files, dimble_path)
    with Pool(16) as p:
        p.starmap(convert, [(dicom_path, dicom_files[i::16], dimble_path) for i in range(16)])

if __name__ == "__main__":
    plac.call(main)
