#!/usr/bin/env bash
set -e

CANISTER=appchain
WASM=target/wasm32-unknown-unknown/release/${CANISTER}.wasm
DID=src/ic/${CANISTER}.did

echo "▶ Building Rust canister $CANISTER..."
cargo build --release --target wasm32-unknown-unknown --package $CANISTER

echo "▶ Extracting candid interface..."
candid-extractor "$WASM" > "$DID"

echo "✅ Generated $DID"
