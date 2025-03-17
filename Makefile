GUEST_RUST_FLAGS="-C relocation-model=pie -C link-arg=--emit-relocs -C link-arg=--unique --remap-path-prefix=$(pwd)= --remap-path-prefix=$HOME=~"

vendor-clone:
	git clone --depth=1 --branch v0.21.0 https://github.com/paritytech/polkavm.git vendor/polkavm
	git clone --depth=1 https://github.com/QuantumFusion-network/polkadot-sdk vendor/polkadot-sdk

tools: polkatool chain-spec-builder

pvm-prog-%:
	cd pvm_prog; RUSTFLAGS=$(GUEST_RUST_FLAGS) cargo build -q --release --bin qf-pvm-$* -p qf-pvm-$*
	mkdir -p output
	polkatool link --run-only-if-newer -s pvm_prog/target/riscv32emac-unknown-none-polkavm/release/qf-pvm-$* -o output/qf-pvm-$*.polkavm

test-pvm-prog-%:
	cd qf-test-runner; cargo run -- --program=../output/qf-pvm-$*.polkavm

chain-spec-builder:
	cargo install --path vendor/polkadot-sdk/substrate/bin/utils/chain-spec-builder

polkatool:
	cargo install --path vendor/polkavm/tools/polkatool

qf-run: qf-release
	output/qf-node --dev --tmp --rpc-cors all

qf-run-wasm: qf-release
	output/qf-node --dev --tmp --rpc-cors all --wasm-runtime-overrides output

qf-node-release: qf-runtime
	cargo build -p qf-node --release
	mkdir -p output
	cp target/release/qf-node output/qf-node

qf-node: qf-runtime
	cargo build -p qf-node
	mkdir -p output
	cp target/debug/qf-node output/qf-node

qf-runtime:
	cargo build -p qf-runtime
	mkdir -p output
	cp target/debug/wbuild/qf-runtime/qf_runtime.wasm output

polkavm-pallet:
	cargo build -p pallet-qf-polkavm-dev

fmt:
	cargo fmt --all

check-wasm:
	SKIP_WASM_BUILD= cargo check --no-default-features --target=wasm32-unknown-unknown -p qf-runtime

check: check-wasm
	SKIP_WASM_BUILD= cargo check

clippy:
	SKIP_WASM_BUILD= cargo clippy -- -D warnings

qf-test:
	SKIP_WASM_BUILD= cargo test

qf-chainspec: qf-runtime
	chain-spec-builder -c output/qf-chainspec.json create -n qf-runtime -i qf-runtime -r ./output/qf_runtime.wasm -s default
	cat output/qf-chainspec.json | jq '.properties = {}' > output/qf-chainspec.json.tmp
	mv output/qf-chainspec.json.tmp output/qf-chainspec.json
