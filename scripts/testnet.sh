#!/usr/bin/env bash

set -e

cargo build
./target/debug/acuity --dev --port 30334 --ws-port 9946 -lruntime=debug
