#!/bin/bash
set -e

echo "> Native..."
cargo check $@
echo "> Android..."
cargo check --target=aarch64-linux-android $@
echo "> Linux..."
cargo check --target=armv7-unknown-linux-gnueabihf $@
echo "> iOS..."
cargo check --target=aarch64-apple-ios $@ 
echo "> FreeBSD..."
cargo check --target=x86_64-unknown-freebsd $@
echo "> NetBSD..."
cargo check --target=x86_64-unknown-netbsd $@
# echo "> Windows..."
# cargo check --target=x86_64-pc-windows-gnu $@
