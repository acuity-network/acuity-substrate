#!/usr/bin/env bash

set -e

cargo build --release
./target/release/acuity --dev --port 30334 --ws-port 9946 -lruntime=debug --pruning archive
