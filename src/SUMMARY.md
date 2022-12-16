# iowatcher-ng documentation

iowatcher-ng is an executable for block I/O which consumes _blktrace_ kernel api.
The executable itself does not produce output such as _blkparse_ or _iowatcher_ but rather
exposes some of the same metrics to Prometheus which dials out to our exporter executable
to scrape them so you can use Grafana, for instance, to graph the values.

# Install

Prerequisites are GCC, G++, Make, Bison, etc. - in Debian systems this is called the _build-essential_ package.
_Clang_ and its development headers, and _llvm_. Finally you can install _blktrace_.

## Preparing Fedora for installation of `Rustup`

sudo the following commands:

```bash

$ dnf install @development-tools cmake

$ dnf install g++ clang clang-devel llvm llvm-devel

$ dnf install blktrace
```

## Installing `Rustup`

1. Install `Rustup` as described [here](https://rustup.rs/).
2. Restart your shell, or source `$HOME/.cargo/env`.
3. Now you have `cargo` in path.

## Compile project

1. Go to your project source directory root and execute `cargo build`.
2. This previous step shall download all dependencies and build the project libraries and executable binaries.

## Development environment

* Install [VSCode](https://code.visualstudio.com/).
* Install [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer) `ext install rust-lang.rust-analyzer`
* Install [Docker for VSCode](https://marketplace.visualstudio.com/items?itemName=ms-azuretools.vscode-docker) `ext install ms-azuretools.vscode-docker`
* Check out project repository in local.
* Open project repository root folder with _VSCode_.