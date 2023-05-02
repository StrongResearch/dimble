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
	pytest

fmt:
	cargo fmt
	isort .
	black .

lint:
	cargo clippy
	flake8
