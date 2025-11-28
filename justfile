# see https://github.com/casey/just

default: build

build:
    cargo build --release
run:
    cargo run --release --example=play-script --features=player

pg-build:
    cd examples/playground && ./build.sh
pg-serve:
    cd examples/playground && ./serve.sh
pg-run: pg-build pg-serve

docs-generate-api:
    cd docs && cargo run
docs-build: docs-generate-api
    cd docs && mdbook build
docs-serve: docs-generate-api
    cd docs && mdbook serve
