#!/bin/bash

# Build the Rust application
cargo build --release

# Create a directory for the bundle
mkdir -p pjplayer_bundle/bin

# Copy the compiled binary and dependencies to the bundle directory
cp target/release/pjplayer pjplayer_bundle/
cp bin/* pjplayer_bundle/bin/

# Create a tarball of the bundle
tar -czvf pjplayer_bundle.tar.gz pjplayer_bundle

# Clean up
rm -rf pjplayer_bundle