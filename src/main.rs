mod engine;
mod parser;
mod types;

use crate::engine::*;

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let eng = Engine::new(args[1].as_str());

    if args.len() == 2 {
        eng.dump_dot();
    } else if args.len() == 3 {
        dbg!(eng.is_match(args[2].as_str()));
    } else {
        panic!(
            "Invalid call with {} args. Do ./bin PATTERN or ./bin PATTERN STRING",
            args.len()
        )
    }
}
