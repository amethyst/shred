# shred - *Sh*ared *re*source *d*ispatcher

[![Build Status][bi]][bl] [![Crates.io][ci]][cl] ![MIT/Apache][li] [![Docs.rs][di]][dl]

[bi]: https://travis-ci.org/torkleyy/shred.svg?branch=master
[bl]: https://travis-ci.org/torkleyy/shred

[ci]: https://img.shields.io/crates/v/shred.svg
[cl]: https://crates.io/crates/shred/

[li]: https://img.shields.io/badge/license-MIT%2FApache-blue.svg

[di]: https://docs.rs/shred/badge.svg
[dl]: https://docs.rs/shred/

Dispatches tasks in parallel which need read access to some resources, 
and write access to others.

## Usage

```rust
extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{DispatcherBuilder, Fetch, FetchMut, Resource, Resources, Task};

#[derive(Debug)]
struct ResA;

impl Resource for ResA {}

#[derive(Debug)]
struct ResB;

impl Resource for ResB {}

#[derive(TaskData)]
struct Data<'a> {
    a: Fetch<'a, ResA>,
    b: FetchMut<'a, ResB>,
}

struct EmptyTask;

impl<'a> Task<'a> for EmptyTask {
    type TaskData = Data<'a>;

    fn work(&mut self, bundle: Data<'a>) {
        println!("{:?}", &*bundle.a);
        println!("{:?}", &*bundle.b);
    }
}


fn main() {
    let mut resources = Resources::new();
    let mut dispatcher = DispatcherBuilder::new()
        .add(EmptyTask, "empty", &[])
        .finish();
    resources.add(ResA, ());
    resources.add(ResB, ());

    dispatcher.dispatch(&mut resources);
}
```

### Required Rust version

`1.15 stable`

## Features

* lock-free
* no channels or similar functionality used
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

See LICENSE-APACHE and LICENSE-MIT.
