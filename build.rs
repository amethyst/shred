extern crate skeptic;

use std::env;

fn main() {
    if env::var("CI") == Ok("true".to_owned()) {
        skeptic::generate_doc_tests(&["README.md"]);
    }
}
