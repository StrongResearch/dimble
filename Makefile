install:
	pip install .[dev]

prep_test:
	python scripts/_make_eye3s.py
	([ ! -d 'pydicom-data' ] && git clone https://github.com/pydicom/pydicom-data.git) || true

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