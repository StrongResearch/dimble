# ruff: noqa: E501
from pathlib import Path

import plac
from tqdm import tqdm

import dimble


@plac.annotations(
    brats_nifti_dir=("Path to BRATS NIFTI directory", "positional", None, Path),
    brats_dimble_dir=("Path to BRATS DIMBLE directory", "positional", None, Path),
)
def main(brats_nifti_dir: Path, brats_dimble_dir: Path):
    # traverse brats dir and convert all nifti files to dimble and save them with same structure in dimble dir
    for nifti_file in tqdm(list(brats_nifti_dir.glob("**/*.nii.gz"))):
        dimble_file = brats_dimble_dir / nifti_file.relative_to(
            brats_nifti_dir
        ).with_suffix(".dimble")
        dimble_file.parent.mkdir(parents=True, exist_ok=True)
        dimble.nifti_to_dimble(nifti_file, dimble_file)


if __name__ == "__main__":
    plac.call(main)
