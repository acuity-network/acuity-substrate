#!/usr/bin/env bash

set -e

#./target/release/acuity purge-chain -y \
#  --base-path staging/$i \
#  --chain staging

./target/release/acuity \
  --base-path staging/$i \
  --chain staging.json \
  --port 3034$i --rpc-port 994$i --ws-port 995$i \
  --name AcuityStaging$i --validator --pruning archive --rpc-cors=all \
  --bootnodes /dns/localhost/tcp/30341/p2p/12D3KooWEmbHPvXo7Q628QMt7cNHexpDWNBhp5DE9yoPYpJyy3ri \
    /dns/localhost/tcp/30342/p2p/12D3KooWKorV9L52JrWbNUi6f5RUwcXdNSoPyUHYMN2zyp9y3uHc \
    /dns/localhost/tcp/30343/p2p/12D3KooWKTgE9YUE1fH4VogXECqubc7C8tjYLJfdCv2txYPENVuP \
    /dns/localhost/tcp/30344/p2p/12D3KooWCR2rJcf3TCt54g7jG1m1u8cUM19WxyRXUS4fmTyiKuNN
