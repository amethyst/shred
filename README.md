# shred - *Sh*ared *re*source *d*ispatcher

[![Build Status][bi]][bl] [![Crates.io][cr]][cl] ![MIT/Apache][li] [![Docs.rs][di]][dl] ![LoC][lo]

[bi]: https://github.com/amethyst/shred/actions/workflows/ci.yml/badge.svg
[bl]: https://github.com/amethyst/shred/actions/workflows/ci.yml

[cr]: https://img.shields.io/crates/v/shred.svg
[cl]: https://crates.io/crates/shred/

[li]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg

[di]: https://docs.rs/shred/badge.svg
[dl]: https://docs.rs/shred/

[lo]: https://tokei.rs/b1/github/amethyst/shred?category=code

This library allows to dispatch
systems, which can have interdependencies,
shared and exclusive resource access, in parallel.

## Usage

```rust
extern crate shred;

use shred::{DispatcherBuilder, Read, Resource, ResourceId, System, SystemData, World, Write};

#[derive(Debug, Default)]
struct ResA;

#[derive(Debug, Default)]
struct ResB;

#[derive(SystemData)] // Provided with `shred-derive` feature
struct Data<'a> {
    a: Read<'a, ResA>,
    b: Write<'a, ResB>,
}

struct EmptySystem;

impl<'a> System<'a> for EmptySystem {
    type SystemData = Data<'a>;

    fn run(&mut self, bundle: Data<'a>) {
        println!("{:?}", &*bundle.a);
        println!("{:?}", &*bundle.b);
    }
}

fn main() {
    let mut world = World::empty();
    let mut dispatcher = DispatcherBuilder::new()
        .with(EmptySystem, "empty", &[])
        .build();
    world.insert(ResA);
    world.insert(ResB);

    dispatcher.dispatch(&mut world);
}
```

Please see [the benchmark](benches/bench.rs) for a bigger (and useful) example.

### Required Rust version

`1.38 stable`

## Features

* lock-free
* no channels or similar functionality used (-> less overhead)
* allows both automated parallelization and fine-grained control

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
