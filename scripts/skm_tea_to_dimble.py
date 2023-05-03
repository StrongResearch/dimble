from pathlib import Path
import dimble
from tqdm import tqdm
import plac

@plac.annotations(
    skm_tea_dicom_path=("Path to SKM-TEA DICOM directory", "positional", None, Path),
    skm_tea_dimble_path=("Path to SKM-TEA DIMBLE directory", "positional", None, Path),
)
def main(skm_tea_dicom_path: Path, skm_tea_dimble_path: Path):
    dicom_files = list(skm_tea_dicom_path.glob("**/*.dcm"))
    for dicom_file in tqdm(dicom_files):
        dimble_file = skm_tea_dimble_path / dicom_file.relative_to(skm_tea_dicom_path).with_suffix(".dimble")
        dimble_file.parent.mkdir(parents=True, exist_ok=True)
        dimble.dicom_to_dimble(dicom_file, dimble_file)

if __name__ == "__main__":
    plac.call(main)