.PHONY: all client server static_server serve_with_python serve_with_rust

all: client server

client:
	wasm-pack build --target web --out-dir static/pkg -- --features=client-deps

server: client
	cargo build --bin server --release --features=server-deps

static_server: client
	cargo build --bin server --release --features=server-deps,server-statically-pack-assets

serve_with_python: client
	(cd static && python -m SimpleHTTPServer)

serve_with_rust: server
	./target/release/server
