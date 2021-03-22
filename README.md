# Acuity Social

Implementation of a https://acuity.social node in Rust based on the Substrate framework.


This repo contains runtimes for the Acuity network. The README provides
information about installing the `acuity` binary and developing on the codebase. For more
specific guides, like how to be a validator, see the
[Polkadot Wiki](https://wiki.polkadot.network/docs/en/).

## Building

### Build from Source

If you'd like to build from source, first install Rust. You may need to add Cargo's bin directory
to your PATH environment variable. Restarting your computer will do this for you automatically.

```bash
curl https://sh.rustup.rs -sSf | sh
```

If you already have Rust installed, make sure you're using the latest version by running:

```bash
rustup update
```

Once done, finish installing the support software:

```bash
sudo apt install build-essential git clang libclang-dev pkg-config libssl-dev
```

Build the client by cloning this repository and running the following commands from the root
directory of the repo:

```bash
git checkout <latest tagged release>
./scripts/init.sh
cargo build --release
```

## Connect to the Acuity Network

Connect to the global Acuity network by running:

```bash
./target/release/acuity
```

## License

Acuity is [GPL 3.0 licensed](LICENSE).
