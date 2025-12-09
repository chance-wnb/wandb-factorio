# Load environment variables from .env file if it exists
ifneq (,$(wildcard .env))
    include .env
    export
endif

.PHONY: build-rust-client run-rust-client run-rust-client-debug

# Build the Rust client
build-rust-client:
	cd rust_client && cargo build --release

# Run the Rust client with W&B tracking
run-rust-client:
	cd rust_client && cargo run

# Run the Rust client with debug logging
run-rust-client-debug:
	cd rust_client && RUST_LOG=debug cargo run
