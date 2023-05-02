from setuptools import find_packages, setup
from setuptools_rust import Binding, RustExtension

setup(
    name="dimble",
    version="0.1.0",
    description="Nimble Digital Imaging for Medicine",
    packages=find_packages(include=["dimble"]),
    install_requires=[
        "setuptools_rust",
        "torch>=1.10",
        "numpy",
        "safetensors",
        "pydicom",
        "plac",
        "msgpack",
        "python-gdcm",
        "SimpleITK",
    ],
    extras_require={"dev": ["black", "flake8", "isort", "pytest"]},
    tests_require=["pytest"],
    rust_extensions=[
        RustExtension("dimble_rs.dimble_rs", binding=Binding.PyO3, debug=False)
    ],
)
