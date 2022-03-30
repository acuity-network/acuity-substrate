#!/usr/bin/env bash

set -e

for i in 1 2 3 4 ; do

  ./target/release/acuity key insert \
    --base-path staging/$i \
		--chain staging \
    --suri //"$SECRET"//babe//$i \
    --key-type babe --scheme Sr25519

  ./target/release/acuity key insert \
    --base-path staging/$i \
		--chain staging \
    --suri //"$SECRET"//grandpa//$i \
    --key-type gran --scheme Ed25519

done;
