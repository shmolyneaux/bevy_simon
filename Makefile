
release:
	cargo build --release --target wasm32-unknown-unknown
	wasm-bindgen --no-typescript --target web --out-dir . --out-name bevy_simon ./target/wasm32-unknown-unknown/release/bevy_simon.wasm
	zip game.zip `find assets/ -type f` index.html bevy_simon_bg.wasm bevy_simon.js

.PHONY: release
