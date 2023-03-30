install:
	pip install .[dev]

prep_test:
	python scripts/_make_eye3s.py

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