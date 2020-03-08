# Installation
This installation manual describes how to install tclscan based on your running tcl version.
 
## Introduction
To complete the instructions experience with a Linux environment rudimentary udnerstanding of coding and some patients are required.

### Environment debian / ubuntu
Install `tcl-dev` and `clang`.
```bash
sudo apt-get install tcl-dev clang
```
### Environment redhat
```bash
sudo subscription-manager repos --enable rhel-7-server-devtools-rpms
sudo subscription-manager repos --enable rhel-server-rhscl-7-rpms
sudo yum install llvm-toolset-7 tcl-devel
```
For more troubleshooting see https://developers.redhat.com/products/clang-llvm-go-rust/hello-world/#fndtn-windows

### Installing rust

Install rustup and cargo from source.
```bash
curl https://sh.rustup.rs -sSf | sh
```
Some of the code relies on features in the nightly build of rust, so you need to switch to it.

```bash
$ rustup toolchain install nightly
info: syncing channel updates for 'nightly-x86_64-unknown-linux-gnu'
$ rustup default nightly
```

Activate the rust environment in your shell

```bash
source ~/.cargo/env
```

### Installing rust-bindgen
Rust-bindgen is a tool that allows you to generate rust bindings from c header files.

Clone the `rust-bindgen` repository.
```bash
git clone https://github.com/rust-lang-nursery/rust-bindgen
```

Check out a recent release
```
git tag

...
v0.42.2
v0.42.3
v0.43.0
v0.43.1
vX.YY.Z

git checkout vX.YY.Z (e.g. the latest branch)
```

Update the cargo and build
```bash
cargo update
cargo build
```

### Creating a build rust tcl header
Prepare the rust to c bindings in two steps:
1. Edit `tclscan/src/mytcl.h` to define the path to `tcl.h` installed through `tcl-dev`/`tcl-devel`.
2. Locate youre `libclang.so` file that you installed from the clang package.

Generate `tcl.rs` in the `tclsca/src/` directory using `bindgen`
```bash
LD_PRELOAD=/usr/lib/llvm-6.0/lib/libclang.so.1 rust-bindgen/target/debug/bindgen -o tclscan/src/tcl.rs tclscan/src/mytcl.h
```

If you get the error

```bash
error: 'rustfmt' is not installed for the toolchain 'nightly-x86_64-unknown-linux-gnu'
```

Simply install rustfmt:

```bash
$ rustup component add rustfmt --toolchain nightly-x86_64-unknown-linux-gnu
```

Upate your running environment
```bash
cargo update
```

Set environment variable for linking against libtcl.so
```bash
export RUSTFLAGS="-C link_args="-ltcl""
```

Compile the program
```bash
cargo build
```

A successfull build produce the executable `tclscan/target/debug/tclscan`.
If you want to install tclscan on other systems, keep in mind that it depends on `libtcl.so` in your `$LD_LIBRARY_PATH`.
