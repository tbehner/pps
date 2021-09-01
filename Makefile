build-container:
	podman build --rm -t rust-musl-x86_64 .

release: build-container
	cargo build --release
	podman run --rm -it -v "${PWD}":/home/rust/src:Z rust-musl-x86_64 cargo build --release
