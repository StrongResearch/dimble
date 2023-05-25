from multiprocessing import Pool
from pathlib import Path

import plac
from tqdm import tqdm

import dimble


def convert(
    dicom_dir: Path, dicom_files: list[Path], dimble_dir: Path
) -> list[tuple[Path, Path]]:
    errors = []
    mappings = []
    for dicom_file in tqdm(dicom_files):
        try:
            dimble_file = dimble_dir / dicom_file.relative_to(dicom_dir).with_suffix(
                ".dimble"
            )
            if dimble_file.exists():
                mappings.append((dicom_file, dimble_file))
                continue
            dimble_file.parent.mkdir(parents=True, exist_ok=True)
            dimble.dicom_to_dimble(dicom_file, dimble_file)
            mappings.append((dicom_file, dimble_file))
        except:
            dimble_file.unlink(missing_ok=True)
            errors.append(dicom_file)
    print(f"Worker finished with {errors=}", flush=True)
    return mappings


def flatten(l):
    return [item for sublist in l for item in sublist]


@plac.annotations(
    dicom_path=("Path to DICOM directory", "positional", None, Path),
    dimble_path=("Path to DIMBLE directory", "positional", None, Path),
)
def main(dicom_path: Path, dimble_path: Path):
    dicom_files = (
        list(dicom_path.glob("**/*.dcm"))
        + list(dicom_path.glob("**/*.dicom"))
        + list(dicom_path.glob("**/*.DCM"))
    )
    # convert(dicom_path, dicom_files, dimble_path)
    with Pool(16) as p:
        mappings = p.starmap(
            convert, [(dicom_path, dicom_files[i::16], dimble_path) for i in range(16)]
        )
    mappings = flatten(mappings)
    with open(str(dicom_path).replace("/", "_") + ".csv", "w") as f:
        for mapping in mappings:
            f.write(str(mapping[0]) + ", " + str(mapping[1]) + "\n")


if __name__ == "__main__":
    plac.call(main)
