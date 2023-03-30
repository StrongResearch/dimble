# dimble
Nimble Digital Imaging for Medicine

## Installation

```sh
# using ssh
git clone git@github.com:StrongCompute/dimble.git
# using https
https://github.com/StrongCompute/dimble.git

cd dimble

pip install .
```



## Usage

```python
import dimble

# convert to dimble
dimble.dicom_to_dimble('xray.dicom', 'xray.dimble')

# load a dimble file
dataset = dimble.load_dimble('xray.dimble', fields=["7FE00010"], device="cpu")

# convert back to dicom
dimble.dimble_to_dicom("xray.dimble", "xray.dicom")
```

## Developing

```sh
pip install .[dev]
```

## Testing

```sh
pip install .[dev]
make test
```