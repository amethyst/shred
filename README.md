# shred - *Sh*ared *re*source *d*ispatcher

[![Build Status][bi]][bl] [![Crates.io][ci]][cl] ![MIT/Apache][li] [![Docs.rs][di]][dl]

[bi]: https://travis-ci.org/torkleyy/shred.svg?branch=master
[bl]: https://travis-ci.org/torkleyy/shred

[ci]: https://img.shields.io/crates/v/shred.svg
[cl]: https://crates.io/crates/shred/

[li]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg

[di]: https://docs.rs/shred/badge.svg
[dl]: https://docs.rs/shred/

This library allows to dispatch
systems, which can have interdependencies,
shared and exclusive resource access, in parallel.

## Usage

```rust
extern crate shred;
#[macro_use]
extern crate shred_derive; // for `#[derive(SystemData)]`

use shred::{DispatcherBuilder, Fetch, FetchMut, Resource, Resources, System};

#[derive(Debug)]
struct ResA;

impl Resource for ResA {}

#[derive(Debug)]
struct ResB;

impl Resource for ResB {}

/// Every system has a predefined
/// system data.
#[derive(SystemData)]
struct PrintData<'a> {
    /// `Fetch` means it reads from
    /// that data
    a: Fetch<'a, ResA>,
    /// `FetchMut` also allows
    /// write access
    b: FetchMut<'a, ResB>,
}

struct PrintSystem;

// Systems should be generic over the
// context if possible, so it's easy
// to introduce one.
impl<'a, C> System<'a, C> for PrintSystem {
    type SystemData = PrintData<'a>;

    fn work(&mut self, mut bundle: PrintData<'a>, _: C) {
        println!("{:?}", &*bundle.a);
        println!("{:?}", &*bundle.b);
        
        *bundle.b = ResB; // We can mutate ResB here
                          // because it's `FetchMut`.
    }
}

fn main() {
    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add(PrintSystem, "print", &[]) // Adds a system "print" without dependencies
        .finish();
    resources.add(ResA, ());
    resources.add(ResB, ());

    // We can even pass a context,
    // but we don't need one here
    // so we pass `()`.
    dispatcher.dispatch(&mut resources, ());
}
```

Please see [the benchmark](benches/bench.rs) for a bigger (and useful) example.

### Required Rust version

`1.15 stable`

## Features

* lock-free
* no channels or similar functionality used (-> less overhead)
* allows lifetimes (opposed to `'static` only)

## Contribution

Contribution is highly welcome! If you'd like another
feature, just create an issue. You can also help
out if you want to; just pick a "help wanted" issue.
If you need any help, feel free to ask!

All contributions are assumed to be dual-licensed under
MIT/Apache-2.

## License

`shred` is distributed under the terms of both the MIT 
license and the Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
