extern crate tclscan;
use std::os;

fn main() {
    let args = os::args();
    tclscan::scan_file(args[1].as_slice());
}
