extern crate shred;
#[macro_use]
extern crate shred_derive;

use shred::{Dispatcher, DispatcherBuilder, Read, Resources, RunningTime, System, Write};

fn sleep_short() {
    use std::thread::sleep;
    use std::time::Duration;

    sleep(Duration::new(0, 1_000));
}

#[derive(Default)]
struct Res;
#[derive(Default)]
struct ResB;

#[derive(SystemData)]
struct DummyData {
    _res: Read<Res>,
}

#[derive(SystemData)]
struct DummyDataMut {
    _res: Write<Res>,
}

struct DummySys;

impl System for DummySys {
    type SystemData = DummyData;

    fn run(&mut self, _data: Self::SystemData) {
        sleep_short()
    }
}

struct DummySysMut;

impl System for DummySysMut {
    type SystemData = DummyDataMut;

    fn run(&mut self, _data: Self::SystemData) {
        sleep_short()
    }
}

struct Whatever<'a>(&'a i32);

impl<'b> System for Whatever<'b> {
    type SystemData = ();

    fn run(&mut self, _: Self::SystemData) {
        println!("{}", self.0);
    }
}

#[test]
fn dispatch_builder() {
    DispatcherBuilder::new()
        .with(DummySys, "a", &[])
        .with(DummySys, "b", &["a"])
        .with(DummySys, "c", &["a"])
        .build();
}

#[test]
#[should_panic(expected = "No such system registered")]
fn dispatch_builder_invalid() {
    DispatcherBuilder::new()
        .with(DummySys, "a", &[])
        .with(DummySys, "b", &["z"])
        .build();
}

#[test]
fn dispatch_basic() {
    let mut res = Resources::new();
    res.insert(Res);

    let number = 5;

    let mut d: Dispatcher = DispatcherBuilder::new()
        .with(DummySys, "a", &[])
        .with(DummySys, "b", &["a"])
        .with(Whatever(&number), "w", &[])
        .build();

    d.dispatch(&mut res);
}

#[test]
fn dispatch_ww_block() {
    let mut res = Resources::new();
    res.insert(Res);

    let mut d: Dispatcher = DispatcherBuilder::new()
        .with(DummySysMut, "a", &[])
        .with(DummySysMut, "b", &[])
        .build();

    d.dispatch(&mut res);
}

#[test]
fn dispatch_rw_block() {
    let mut res = Resources::new();
    res.insert(Res);

    let mut d: Dispatcher = DispatcherBuilder::new()
        .with(DummySys, "a", &[])
        .with(DummySysMut, "b", &[])
        .build();

    d.dispatch(&mut res);
}

#[test]
fn dispatch_rw_block_rev() {
    let mut res = Resources::new();
    res.insert(Res);

    let mut d: Dispatcher = DispatcherBuilder::new()
        .with(DummySysMut, "a", &[])
        .with(DummySys, "b", &[])
        .build();

    d.dispatch(&mut res);
}

#[test]
fn dispatch_sequential() {
    let mut res = Resources::new();
    res.insert(Res);

    let mut d: Dispatcher = DispatcherBuilder::new()
        .with(DummySysMut, "a", &[])
        .with(DummySys, "b", &[])
        .build();

    d.dispatch_seq(&mut res);
}

#[cfg(feature = "parallel")]
#[test]
fn dispatch_async() {
    let mut res = Resources::new();
    res.insert(Res);

    let mut d = DispatcherBuilder::new()
        .with(DummySysMut, "a", &[])
        .with(DummySys, "b", &[])
        .build_async(res);

    d.dispatch();

    d.wait();
}

#[cfg(feature = "parallel")]
#[test]
fn dispatch_async_res() {
    let mut res = Resources::new();
    res.insert(Res);

    let mut d = DispatcherBuilder::new()
        .with(DummySysMut, "a", &[])
        .with(DummySys, "b", &[])
        .build_async(res);

    d.dispatch();

    let res = d.mut_res();
    res.insert(ResB);
}

#[test]
fn dispatch_stage_group() {
    let mut res = Resources::new();
    res.insert(Res);
    res.insert(ResB);

    struct ReadingFromResB;

    impl System for ReadingFromResB {
        type SystemData = Read<ResB>;

        fn run(&mut self, _: Self::SystemData) {
            sleep_short()
        }

        fn running_time(&self) -> RunningTime {
            RunningTime::Short
        }
    }

    struct WritingToResB;

    impl System for WritingToResB {
        type SystemData = Write<ResB>;

        fn run(&mut self, _: Self::SystemData) {
            sleep_short()
        }

        fn running_time(&self) -> RunningTime {
            RunningTime::VeryShort
        }
    }

    let mut d: Dispatcher = DispatcherBuilder::new()
        .with(DummySys, "read_a", &[])
        .with(ReadingFromResB, "read_b", &[])
        .with(WritingToResB, "write_b", &[])
        .build();

    d.dispatch(&mut res);
}
