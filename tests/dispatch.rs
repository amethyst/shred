extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{Dispatcher, DispatcherBuilder, Fetch, FetchMut, Resources, System};

#[derive(Debug)]
struct Res;

#[derive(SystemData)]
struct DummyData<'a> {
    _res: Fetch<'a, Res>,
}

#[derive(SystemData)]
struct DummyDataMut<'a> {
    _res: FetchMut<'a, Res>,
}

struct DummySys;

impl<'a> System<'a> for DummySys {
    type SystemData = DummyData<'a>;

    fn work(&mut self, _data: Self::SystemData) {}
}

struct DummySysMut;

impl<'a> System<'a> for DummySysMut {
    type SystemData = DummyDataMut<'a>;

    fn work(&mut self, _data: Self::SystemData) {}
}

#[test]
fn dispatch_builder() {
    DispatcherBuilder::new()
        .add(DummySys, "a", &[])
        .add(DummySys, "b", &["a"])
        .add(DummySys, "c", &["a"])
        .build();
}

#[test]
#[should_panic(expected = "No such system registered")]
fn dispatch_builder_invalid() {
    DispatcherBuilder::new()
        .add(DummySys, "a", &[])
        .add(DummySys, "b", &["z"])
        .build();
}

#[test]
fn dispatch_basic() {
    let mut res = Resources::new();
    res.add(Res, ());

    let mut d: Dispatcher = DispatcherBuilder::new()
        .add(DummySys, "a", &[])
        .add(DummySys, "b", &["a"])
        .build();

    d.dispatch(&mut res);
}

#[test]
fn dispatch_rw_block() {
    let mut res = Resources::new();
    res.add(Res, ());

    let mut d: Dispatcher = DispatcherBuilder::new()
        .add(DummySys, "a", &[])
        .add(DummySysMut, "b", &[])
        .build();

    d.dispatch(&mut res);
}

#[test]
fn dispatch_rw_block_rev() {
    let mut res = Resources::new();
    res.add(Res, ());

    let mut d: Dispatcher = DispatcherBuilder::new()
        .add(DummySysMut, "a", &[])
        .add(DummySys, "b", &[])
        .build();

    d.dispatch(&mut res);
}

#[test]
fn dispatch_sequential() {
    let mut res = Resources::new();
    res.add(Res, ());

    let mut d: Dispatcher = DispatcherBuilder::new()
        .add(DummySysMut, "a", &[])
        .add(DummySys, "b", &[])
        .build();

    d.dispatch_seq(&mut res);
}

#[cfg(feature = "parallel")]
#[test]
fn dispatch_async() {
    let mut res = Resources::new();
    res.add(Res, ());

    let mut d = DispatcherBuilder::new()
        .add(DummySysMut, "a", &[])
        .add(DummySys, "b", &[])
        .build_async(res);

    d.dispatch();

    d.wait();
}

#[cfg(feature = "parallel")]
#[test]
fn dispatch_async_res() {
    let mut res = Resources::new();
    res.add(Res, ());

    let mut d = DispatcherBuilder::new()
        .add(DummySysMut, "a", &[])
        .add(DummySys, "b", &[])
        .build_async(res);

    d.dispatch();

    let res = d.mut_res();
    res.add(Res, 2);
}
