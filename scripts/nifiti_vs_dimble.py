# ruff: noqa: E501
import time
from pathlib import Path
from typing import Optional

import numpy as np
import plac
import SimpleITK as sitk
import torch
from tqdm import tqdm

import dimble


def dimble_loading(dimble_files, cuda=False, verbose=True):
    for file in tqdm(dimble_files, desc="DIMBLE", disable=not verbose):
        data = dimble.load_dimble(file, ["7FE00010"])
        if cuda:
            data = data["7FE00010"].cuda()


def nifti_loading(nifti_files, cuda=False, verbose=True):
    for file in tqdm(nifti_files, desc="NIFTI", disable=not verbose):
        data = sitk.ReadImage(str(file))
        data = sitk.GetArrayFromImage(data)
        data = torch.from_numpy(data.astype(np.int32))
        if cuda:
            data = data.cuda()


@plac.annotations(
    nifti_dir=("Path to NIFTI directory", "positional", None, Path),
    dimble_dir=("Path to DIMBLE directory", "positional", None, Path),
    n=("Number of files to load", "option", "n", int),
    cuda=("Use CUDA", "flag", "c", bool),
)
def main(nifti_dir: Path, dimble_dir: Path, n: Optional[int], cuda: bool = False):
    if n is None:
        n = -1  # load all
    nifti_files = list(nifti_dir.rglob("*.nii.gz"))[:n]
    dimble_files = list(dimble_dir.rglob("*.dimble"))[:n]
    assert len(dimble_files) == len(
        nifti_files
    ), "Number of files in NIFTI and DIMBLE directories must match"
    print("Loading", len(nifti_files), "files")

    # warmup
    dimble_loading(dimble_files[:5], cuda=cuda, verbose=False)
    dimble_loading(dimble_files[:5], cuda=cuda, verbose=False)
    torch.cuda.synchronize()

    # NIFTI
    nifti_start = time.perf_counter()
    nifti_loading(nifti_files, cuda=cuda)
    torch.cuda.synchronize()
    nifti_end = time.perf_counter()
    nifti_elapsed = nifti_end - nifti_start

    # DIMBLE
    dimble_start = time.perf_counter()
    dimble_loading(dimble_files, cuda=cuda)
    torch.cuda.synchronize()
    dimble_end = time.perf_counter()
    dimble_elapsed = dimble_end - dimble_start

    # RESULTS
    print()
    print(
        f"NIFTI loaded {len(nifti_files)} files in {nifti_elapsed:.4f} seconds, {len(nifti_files)/nifti_elapsed:.2f} images/second"
    )
    print(
        f"DIMBLE loaded {len(dimble_files)} files in {dimble_elapsed:.4f} seconds, {len(dimble_files)/dimble_elapsed:.2f} images/second"
    )
    print(f"Speedup: {nifti_elapsed/dimble_elapsed:.2f}x")


if __name__ == "__main__":
    plac.call(main)
