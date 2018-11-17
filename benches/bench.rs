#![feature(test)]

extern crate cgmath;
extern crate shred;
#[macro_use]
extern crate shred_derive;
extern crate test;

use std::ops::{Index, IndexMut};

use cgmath::Vector3;
use shred::*;
use test::Bencher;

#[derive(Debug)]
struct VecStorage<T> {
    data: Vec<T>,
}

impl<T: Clone> VecStorage<T> {
    fn new(init: T) -> Self {
        VecStorage {
            data: vec![init; NUM_COMPONENTS],
        }
    }
}

impl<T> Default for VecStorage<T> {
    fn default() -> Self {
        VecStorage { data: vec![] }
    }
}

impl<T> Index<usize> for VecStorage<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> IndexMut<usize> for VecStorage<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

#[derive(Debug)]
struct DeltaTime(f32);

type Vec3 = Vector3<f32>;

#[derive(Clone, Debug)]
struct Mass(f32);
#[derive(Clone, Debug)]
struct Pos(Vec3);
#[derive(Clone, Debug)]
struct Vel(Vec3);
#[derive(Clone, Debug)]
struct Force(Vec3);

#[derive(Clone, Copy, Debug)]
struct Spring {
    /// the index of the other "entity"
    connection_to: usize,
    constant: f32,
    rest: f32,
}

type MassStorage = VecStorage<Mass>;
type PosStorage = VecStorage<Pos>;
type VelStorage = VecStorage<Vel>;
type ForceStorage = VecStorage<Force>;
type SpringStorage = VecStorage<Spring>;

const NUM_COMPONENTS: usize = 200;

// --------------

#[derive(SystemData)]
struct SpringForceData {
    pos: Read<PosStorage>,
    spring: Read<SpringStorage>,

    force: Write<ForceStorage>,
}

struct SpringForce;

impl System for SpringForce {
    type SystemData = SpringForceData;

    fn run(&mut self, mut data: SpringForceData) {
        for elem in 0..NUM_COMPONENTS {
            let pos = data.pos[elem].0;
            let spring: Spring = data.spring[elem];
            let other_pos = data.pos[spring.connection_to].0;

            let force = pos - other_pos;

            let len = (force.x * force.x + force.y * force.y + force.z * force.z).sqrt();
            let magn = (len - spring.rest).abs() * spring.constant;

            let mul = -magn / len;

            let force = force * mul;
            data.force[elem].0 += force;
        }
    }
}

#[derive(SystemData)]
struct IntegrationData {
    force: Read<ForceStorage>,
    mass: Read<MassStorage>,
    pos: Write<PosStorage>,
    vel: Write<VelStorage>,

    time: Option<Read<DeltaTime>>,
}

struct IntegrationSystem;

impl System for IntegrationSystem {
    type SystemData = IntegrationData;

    fn run(&mut self, mut data: IntegrationData) {
        let delta = match data.time {
            Some(time) => time.0,
            None => return,
        };

        for elem in 0..NUM_COMPONENTS {
            let mass = data.mass[elem].0;

            if mass == 0.0 {
                // infinite mass
                continue;
            }

            let pos = &mut data.pos[elem].0;
            let vel = data.vel[elem].0;

            *pos = vel * delta;

            let force = data.force[elem].0;

            let vel = vel + (force / mass) * delta;

            let damping = (0.9f32).powf(delta);
            let vel = vel * damping;
            data.vel[elem] = Vel(vel);
        }
    }
}

#[derive(SystemData)]
struct ClearForceAccumData {
    force: Write<ForceStorage>,
}

struct ClearForceAccum;

impl System for ClearForceAccum {
    type SystemData = ClearForceAccumData;

    fn run(&mut self, mut data: ClearForceAccumData) {
        for elem in 0..NUM_COMPONENTS {
            data.force[elem] = Force(Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            });
        }
    }
}

#[bench]
fn basic(b: &mut Bencher) {
    let mut dispatcher = DispatcherBuilder::new()
        .with(SpringForce, "spring", &[])
        .with(IntegrationSystem, "integration", &[])
        .with(ClearForceAccum, "clear_force", &["integration"]) // clear_force is executed after
                                                               // the integration
        .build();

    let mut res = Resources::new();
    let mass = VecStorage::new(Mass(10.0));
    let mut pos = VecStorage::new(Pos(Vec3::new(0.0, 0.0, 0.0)));
    let vel = VecStorage::new(Vel(Vec3::new(0.0, 0.0, 0.0)));
    let force = VecStorage::new(Force(Vec3::new(0.0, 0.0, 0.0)));
    let spring = VecStorage::new(Spring {
        constant: 2.0,
        connection_to: 0,
        rest: 1.0,
    });

    pos.data[0] = Pos(Vec3::new(-5.0, -5.0, -5.0));

    res.insert(DeltaTime(0.05));
    res.insert(mass);
    res.insert(pos);
    res.insert(vel);
    res.insert(force);
    res.insert(spring);

    b.iter(|| dispatcher.dispatch(&mut res));
}
