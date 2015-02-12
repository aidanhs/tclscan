#![feature(plugin)]
#![feature(io)]
#![feature(path)]
#![feature(core)]

extern crate "rustc-serialize" as rustc_serialize;
extern crate docopt;
#[plugin] #[no_link] extern crate docopt_macros;
extern crate tclscan;

use std::old_io;
use docopt::Docopt;
use tclscan::rstcl;

docopt!(Args derive Show, "
Usage: tclscan check ( - | <path> )
       tclscan parsestr ( - | <script-str> )
");

pub fn main() {
    let args: Args = Args::docopt().decode().unwrap_or_else(|e| e.exit());
    let take_stdin = args.cmd__;
    let script_in = match (args.cmd_check, args.cmd_parsestr, take_stdin) {
        (true, false, false) => {
            let path = &args.arg_path[];
            let mut file = old_io::File::open(&Path::new(path));
            let read_result = file.read_to_string();
            if read_result.is_err() {
                println!("WARN: Couldn't read {}: {}", path, read_result.unwrap_err());
                return;
            }
            read_result.unwrap()
        },
        (true, false, true) |
        (false, true, true) => old_io::stdin().read_to_string().unwrap(),
        (false, true, false) => args.arg_script_str,
        _ => panic!("Internal error: could not load script"),
    };
    let script = &script_in[];
    match (args.cmd_check, args.cmd_parsestr) {
        (true, false) => {
            let results = tclscan::scan_script(script);
            if results.len() > 0 {
                for check_result in results.iter() {
                    println!("{}", check_result);
                }
                println!("");
            };
        },
        (false, true) =>
            println!("{:?}", rstcl::parse_command(script)),
        _ =>
            panic!("Internal error: invalid operation"),
    }
}
