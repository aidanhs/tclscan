extern crate tclscan;
use std::os;
use tclscan::rstcl;

static HELP: &'static str =
r"tclscan file <path>
tclscan str <string>";

pub fn main() {
    let args = os::args();
    match args.as_slice() {
        [_, ref op, ref arg] if *op == "file" => {
            tclscan::scan_file(arg.as_slice());
        },
        [_, ref op, ref arg] if *op == "str" => {
            println!("{:?}", rstcl::parse_command(arg.as_slice()));
        },
        _ => println!("{}", HELP)
    };
}
