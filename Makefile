GUEST_RUST_FLAGS="-C relocation-model=pie -C link-arg=--emit-relocs -C link-arg=--unique --remap-path-prefix=$(pwd)= --remap-path-prefix=$HOME=~"

vendor-clone:
	git clone --depth=1 https://github.com/acalanetwork/polkavm vendor/polkavm
	git clone --depth=1 https://github.com/QuantumFusion-network/polkadot-sdk vendor/polkadot-sdk

tools: polkatool chain-spec-builder

chain-spec-builder:
	cargo install --path vendor/polkadot-sdk staging-chain-spec-builder

polkatool:
	cargo install --path vendor/polkavm/tools/polkatool

qf-run: qf-node
	output/qf-node --dev --tmp --rpc-cors all

qf-run-wasm: qf-node
	output/qf-node --dev --tmp --rpc-cors all --wasm-runtime-overrides output

qf-node: qf-runtime
	cargo build -p qf-node
	mkdir -p output
	cp target/debug/qf-node output

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

chainspec: qf-runtime
	chain-spec-builder -c output/chainspec.json create -n qf-runtime -i qf-runtime -r ./output/qf_runtime.wasm -s default
	cat output/chainspec.json | jq '.properties = {}' > output/chainspec.json.tmp
	mv output/chainspec.json.tmp output/chainspec.json