//! This example demonstrates how to define run-time resources, i.e. multiple
//! resources with the same static type. Such a pattern is useful for scripting,
//! but it's generally not recommended to use this to define multiple
//! resources of a standard Rust type.
//!
//! For Specs (https://github.com/slide-rs/specs) users:
//!
//! For example, having multiple `Camera` resources in a 3D simulation would be
//! considered an anti-pattern. Instead, you should use multiple components to
//! achieve your goal of multiple cameras.
//!
//! The code in this example is structured into multiple steps which hopefull
//! make it easier to understand. Make sure you understood a step before you go
//! to the next.

use hashbrown::HashMap;
use shred::{Accessor, AccessorCow, DynamicSystemData, Fetch, ResourceId, RunNow, System, World};

// -- Step 1 - Define your resource type and an interface for registering it --

#[derive(Debug)]
pub struct ScriptableResource {
    fields: HashMap<String, String>,
}

#[derive(Debug)]
pub struct ScriptingInterface {
    id_alloc: u64,
    type_map: HashMap<String, u64>,
}

impl ScriptingInterface {
    pub fn new() -> Self {
        ScriptingInterface {
            id_alloc: 1, /* Start with `1` so systems don't fetch it accidentally (via
                          * `Fetch<ScriptingResource>`) */
            type_map: HashMap::new(),
        }
    }

    /// Registers a run-time resource as `name` and adds it to `world`.
    pub fn add_rt_resource(&mut self, name: &str, res: ScriptableResource, world: &mut World) {
        self.type_map.insert(name.to_owned(), self.id_alloc);
        self.id_alloc += 1;

        let id = self.resource_id(name).unwrap();
        world.insert_internal(id, res);
    }

    pub fn remove_rt_resource(
        &mut self,
        name: &str,
        world: &mut World,
    ) -> Option<ScriptableResource> {
        let id = self.type_map.remove(name);

        id.and_then(|id| {
            world.remove_internal(ResourceId::new_with_dynamic_id::<ScriptableResource>(id))
        })
    }

    pub fn clear_rt_resources(&mut self, world: &mut World) {
        for &dynamic_id in self.type_map.values() {
            world.remove_internal::<ScriptableResource>(ResourceId::new_with_dynamic_id::<
                ScriptableResource,
            >(dynamic_id));
        }

        self.type_map.clear();
        self.id_alloc = 1;
    }

    /// Returns the resource ID for the dynamic type identified by `name`
    pub fn resource_id(&self, name: &str) -> Option<ResourceId> {
        self.type_map
            .get(name)
            .cloned()
            .map(ResourceId::new_with_dynamic_id::<ScriptableResource>)
    }
}

// -- Step 2 - Setup the World --

fn setup_world() -> World {
    let mut world = World::new();

    let mut interface = ScriptingInterface::new();

    interface.add_rt_resource(
        "Foo",
        ScriptableResource {
            fields: vec![("foo_field".to_owned(), "5".to_owned())]
                .into_iter()
                .collect(),
        },
        &mut world,
    );

    // Make it accessible via the world
    world.insert(interface);

    world
}

// -- Step 3 - Preparations for fetching `ScriptingResource` from systems --

pub struct ScriptingResAccessor {
    reads: Vec<ResourceId>,
    // could also add `writes` here
}

impl ScriptingResAccessor {
    pub fn new(reads: &[&str], world: &World) -> Self {
        let interface = world.fetch::<ScriptingInterface>();

        ScriptingResAccessor {
            reads: reads
                .into_iter()
                .flat_map(|&name| interface.resource_id(name))
                .collect(),
        }
    }
}

impl Accessor for ScriptingResAccessor {
    fn try_new() -> Option<Self> {
        None
    }

    fn reads(&self) -> Vec<ResourceId> {
        self.reads.clone()
    }

    fn writes(&self) -> Vec<ResourceId> {
        vec![]
    }
}

pub struct ScriptingResData<'a> {
    reads: Vec<Fetch<'a, ScriptableResource>>,
}

impl<'a> DynamicSystemData<'a> for ScriptingResData<'a> {
    type Accessor = ScriptingResAccessor;

    fn setup(_accessor: &Self::Accessor, _world: &mut World) {}

    fn fetch(access: &ScriptingResAccessor, world: &'a World) -> Self {
        ScriptingResData {
            reads: access
                .reads
                .iter()
                .map(|id| {
                    world
                        .try_fetch_internal(id.clone())
                        .expect("Resource no longer exists")
                        .borrow()
                })
                .map(|r| unsafe { Fetch::from_inner_unchecked(r) })
                .collect(),
        }
    }
}

// -- Step 4 - Actually defining a system --

struct MySys {
    accessor: ScriptingResAccessor,
}

impl<'a> System<'a> for MySys {
    type SystemData = ScriptingResData<'a>;

    fn run(&mut self, data: Self::SystemData) {
        for scripting_resource in data.reads {
            println!(
                "Fields of run-time resource: {:?}",
                scripting_resource.fields
            );
        }
    }

    fn accessor<'b>(&'b self) -> AccessorCow<'a, 'b, Self> {
        AccessorCow::Ref(&self.accessor)
    }
}

// -- Step 5 - Putting things together --

fn main() {
    let world = setup_world();

    let mut my_system = MySys {
        accessor: ScriptingResAccessor::new(&["Foo"], &world),
    };

    my_system.run_now(&world);
}
