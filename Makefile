default: build

build:
	cargo build --release

install: build
	cp target/release/docker-mon ~/bin
