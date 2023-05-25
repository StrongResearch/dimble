# ruff: noqa: E501
import json
import time
from pathlib import Path
from typing import Callable, Optional

import numpy as np
import pandas as pd
import plac
import pydicom
import SimpleITK as sitk
import torch
from tqdm import tqdm

import dimble


def dimble_loader(file: Path, task: str) -> torch.Tensor:
    match task:
        case "load":
            return dimble.load_dimble(file, ["7FE00010"])["7FE00010"]
        case "sum":
            data = dimble.load_dimble(file, ["7FE00010"])["7FE00010"]
            return data.sum()
        case "rrc_sum":
            return dimble.load_dimble(
                file, ["7FE00010"], slices=[slice(0, 224), slice(0, 224)]
            )["7FE00010"].sum()
    raise ValueError


def dicom_loader(file: Path, task: str) -> torch.Tensor:
    data = pydicom.dcmread(str(file))
    data = torch.from_numpy(data.pixel_array.astype(np.int32))

    match task:
        case "load":
            return data
        case "sum":
            return data.sum()
        case "rrc_sum":
            return data[..., :224, :224].sum()
    raise ValueError


def nifti_loader(file: Path, task) -> torch.Tensor:
    data = sitk.ReadImage(str(file))
    data = sitk.GetArrayFromImage(data)
    data = torch.from_numpy(data.astype(np.int32))

    match task:
        case "load":
            return data
        case "sum":
            return data.sum()
        case "rrc_sum":
            return data[..., :224, :224].sum()
    raise ValueError


loaders = {"dimble": dimble_loader, "dicom": dicom_loader, "nifti": nifti_loader}


def file_loading(
    loader: Callable[[Path, str], torch.Tensor],
    task: str,
    files: list[Path],
    cuda=False,
    verbose=True,
    desc=None,
) -> list[int]:
    times = []
    for file in tqdm(files, desc=desc, disable=not verbose):
        start_time = time.perf_counter()
        data = loader(file, task)
        if cuda:
            data = data.cuda()

        end_time = time.perf_counter()
        elapsed_time = end_time - start_time
        times.append(elapsed_time)
    return times


def prepare_paths(mappings_file: Path, n: int) -> tuple[list[Path], list[Path]]:
    df = pd.read_csv(mappings_file, header=None).iloc[:n]
    original_paths = df[0].apply(lambda p: Path(p.strip())).to_list()
    dimble_paths = df[1].apply(lambda p: Path(p.strip())).to_list()
    return original_paths, dimble_paths


@plac.pos("mode", "DICOM or NIfTI", choices=["dicom", "nifti"])
@plac.pos("mappings_file", "Path to mappings file", type=Path)
@plac.opt("n", "Number of files to load", type=int)
@plac.flg("cuda", "Use CUDA")
def main(mode, mappings_file: Path, n: Optional[int], cuda: bool = False):
    if n is None:
        n = -1
    original_files, dimble_files = prepare_paths(mappings_file, n)
    print("Loading", len(original_files), "files")

    original_loader = loaders[mode]

    tasks = ["load", "sum", "rrc_sum"]

    timings_dict: dict[str, list[float]] = {}
    for name, loader, files in [
        ("original", original_loader, original_files),
        ("dimble", dimble_loader, dimble_files),
    ]:
        for task in tasks:
            timings = file_loading(loader, task, files, cuda=cuda, verbose=False)
            timings_dict[f"{name}-{task}"] = timings

    with open(f"{mappings_file.stem}_timings.json", "w") as f:
        json.dump(timings_dict, f, indent=2)


if __name__ == "__main__":
    plac.call(main)
