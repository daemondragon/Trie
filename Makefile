all:
	cargo build --release
	mv target/release/TextMiningCompiler .
	mv target/release/TextMiningApp .

clean:
	cargo clean