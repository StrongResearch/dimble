from multiprocessing import Pool
from pathlib import Path

import plac
from tqdm import tqdm

import dimble

converters = {"dicom": dimble.dicom_to_dimble, "nifti": dimble.nifti_to_dimble}


def convert(
    input_dir: Path, input_files: list[Path], dimble_dir: Path
) -> list[tuple[Path, Path]]:
    errors = []
    mappings = []
    for input_file in tqdm(input_files):
        name = input_file.name
        if name.endswith(".nii.gz"):
            converter = converters["nifti"]
        else:
            converter = converters["dicom"]
        try:
            dimble_file = dimble_dir / input_file.relative_to(input_dir).with_suffix(
                ".dimble"
            )
            if dimble_file.exists():
                mappings.append((input_file, dimble_file))
                continue
            dimble_file.parent.mkdir(parents=True, exist_ok=True)
            converter(input_file, dimble_file)
            mappings.append((input_file, dimble_file))
        except:
            dimble_file.unlink(missing_ok=True)
            errors.append(input_file)
    print(f"Worker finished with {errors=}", flush=True)
    return mappings


def get_input_files(input_path: Path, extensions: list[str]) -> list[Path]:
    input_files = []
    for extension in extensions:
        files = list(input_path.glob(f"**/*.{extension}"))
        input_files.extend(files)
    return input_files


def flatten(l):
    return [item for sublist in l for item in sublist]


@plac.pos("input_path", "Path to DICOM or NiFTI directory", type=Path)
@plac.pos("dimble_path", "Path to DIMBLE directory", type=Path)
def main(input_path: Path, dimble_path: Path):
    extensions = ["dcm", "dicom", "DCM", "nii.gz"]
    input_files = get_input_files(input_path, extensions)

    with Pool(16) as p:
        mappings = p.starmap(
            convert, [(input_path, input_files[i::16], dimble_path) for i in range(16)]
        )
    mappings = flatten(mappings)
    with open(str(input_path).replace("/", "_") + ".csv", "w") as f:
        for mapping in mappings:
            f.write(str(mapping[0]) + ", " + str(mapping[1]) + "\n")


if __name__ == "__main__":
    plac.call(main)
