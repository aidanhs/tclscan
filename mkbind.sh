cd ~/rust/rust-bindgen/target/debug
LD_PRELOAD=/usr/lib/llvm-3.4/lib/libclang.so ./bindgen -ltcl -builtins \
    -o $(cd -)/src/tcl.rs $(cd -)/src/mytcl.h
