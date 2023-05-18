# ruff: noqa: E501
import time
from pathlib import Path
from typing import Optional
import json

import numpy as np
import plac
import pydicom
import torch
from tqdm import tqdm

import dimble


def dimble_loading(dimble_files, cuda=False, verbose=True) -> list[int]:
    times = []
    for file in tqdm(dimble_files, desc="DIMBLE", disable=not verbose):
        start_time = time.perf_counter()
        data = dimble.load_dimble(file, ["7FE00010"])
        if cuda:
            data = data["7FE00010"].cuda()
        end_time = time.perf_counter()
        elapsed_time = end_time - start_time
        times.append(elapsed_time)
    return times


def dicom_loading(dicom_files, cuda=False, verbose=True) -> list[int]:
    times = []
    for file in tqdm(dicom_files, desc="DICOM", disable=not verbose):
        start_time = time.perf_counter()
        data = pydicom.dcmread(str(file))
        data = torch.from_numpy(data.pixel_array.astype(np.int32))
        if cuda:
            data = data.cuda()
        end_time = time.perf_counter()
        elapsed_time = end_time - start_time
        times.append(elapsed_time)
    return times

@plac.annotations(
    dicom_dir=("Path to DICOM directory", "positional", None, Path),
    dimble_dir=("Path to DIMBLE directory", "positional", None, Path),
    n=("Number of files to load", "option", "n", int),
    cuda=("Use CUDA", "flag", "c", bool),
)
def main(dicom_dir: Path, dimble_dir: Path, n: Optional[int], cuda: bool = False):
    if n is None:
        n = -1
    dicom_files = dimble.rglob_dicom(dicom_dir)[:n]
    dimble_files = list(dimble_dir.rglob("*.dimble"))[:n]
    assert len(dimble_files) == len(
        dicom_files
    ), "Number of files in DICOM and DIMBLE directories must match"
    print("Loading", len(dicom_files), "files")

    # warmup
    dimble_loading(dimble_files[:5], cuda=cuda, verbose=False)
    dimble_loading(dimble_files[:5], cuda=cuda, verbose=False)
    torch.cuda.synchronize()

    # DICOM
    dicom_start = time.perf_counter()
    dicom_times = dicom_loading(dicom_files, cuda=cuda)
    torch.cuda.synchronize()
    dicom_end = time.perf_counter()
    dicom_elapsed = dicom_end - dicom_start

    # DIMBLE
    dimble_start = time.perf_counter()
    dimble_times = dimble_loading(dimble_files, cuda=cuda)
    torch.cuda.synchronize()
    dimble_end = time.perf_counter()
    dimble_elapsed = dimble_end - dimble_start

    # RESULTS
    print()
    print(
        f"DICOM loaded {len(dicom_files)} files in {dicom_elapsed:.2f} seconds, {len(dicom_files)/dicom_elapsed:.2f} images/second"
    )
    print(
        f"DIMBLE loaded {len(dimble_files)} files in {dimble_elapsed:.2f} seconds, {len(dimble_files)/dimble_elapsed:.2f} images/second"
    )
    print(f"DIMBLE is {dicom_elapsed/dimble_elapsed:.2f}x faster than DICOM")

    with open("/tmp/dicom_vs_dimble_timings.json", 'w') as f:
        json.dump(
            {
                "dicom_times": dicom_times,
                "dimble_times": dimble_times,
            },
            f
        )


if __name__ == "__main__":
    plac.call(main)
