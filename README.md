# dimble

Nimble Digital Imaging for Medicine

## Pipeline

### Completed
[x] Near lossless and easy conversion from DICOM and back
[x] Support for fast and random access of metadata
[x] Extremely fast and zero-copy loading to CPU/GPU
[x] Safe: no codegen/exec based on the metadata
[x] Support for ITK file formats [[ref](https://simpleitk.readthedocs.io/en/v1.2.4/Documentation/docs/source/IO.html#images)], including NIfTI

### WIP
[ ] All relevant data types including uint16, f16, bf16, complex64 and complex128
- Currently supports f32, trivial to support other datatypes

[ ] Bindings for Python and conversion to NumPy/CuPy/JAX/Torch tensors
- Currently supports loading to Torch tensors (easily extensible)


## Installation

```sh
# using ssh
git clone git@github.com:StrongCompute/dimble.git
# OR using https
git clone https://github.com/StrongCompute/dimble.git

cd dimble

make install
make validate_install
```


## Usage

```python
import dimble

# convert to dimble
dimble.dicom_to_dimble('xray.dicom', 'xray.dimble')

# load a dimble file's pixel data
dataset = dimble.load_dimble('xray.dimble', fields=["7FE00010"], device="cpu")

# load a dimble file's pixel data sliced to a 224x224 chunk offset by 100 in each dimension
dataset = dimble.load_dimble('xray.dimble', fields=["7FE00010"], device="cpu", slices=[slice(100,100+224), slice(100,100+224)])

# convert back to dicom
dimble.dimble_to_dicom("xray.dimble", "xray.dicom")
```


## Developing

```sh
make install-dev
```


## Testing

```sh
make install-dev
make test
```
