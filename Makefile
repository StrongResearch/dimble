install:
	pip install .

install-dev:
	pip install .[dev]

validate_install:
	python scripts/validate_install

prep_test:
	python scripts/_make_eye3s.py
	([ ! -d 'pydicom-data' ] && git clone https://github.com/pydicom/pydicom-data.git) || true
	([ ! -d 'niivue-images' ] && git clone https://github.com/neurolabusc/niivue-images.git) || true

test: prep_test
	cargo test
	pytest --benchmark-disable

bench: prep_test
	pytest --benchmark-enable --benchmark-group-by=fullfunc

bench-dicom-baselines:
	pytest test/dicom_baseline_comparisons.py --benchmark-group-by=param -s --benchmark-save=dicom_baselines_comparisons

bench-nifti-baselines:
	pytest test/nifti_baseline_comparisons.py --benchmark-group-by=param -s --benchmark-save=nifti_baselines_comparisons

fmt:
	cargo fmt
	isort .
	black .

lint:
	cargo clippy
	flake8
