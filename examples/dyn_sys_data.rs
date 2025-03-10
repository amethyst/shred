//! Warning: this example is a lot more advanced than the others.
//!
//! This example shows a possible way to make shred interact with a scripting
//! language.
//!
//! It does that by implementing `DynamicSystemData` and using `MetaTable`.
#![allow(clippy::disallowed_names)]

extern crate shred;

// faster alternative to std's HashMap
use ahash::AHashMap as HashMap;

use shred::{
    Accessor, AccessorCow, CastFrom, DispatcherBuilder, DynamicSystemData, MetaTable, Read,
    Resource, ResourceId, System, SystemData, World,
    cell::{AtomicRef, AtomicRefMut},
};

struct Dependencies {
    reads: Vec<ResourceId>,
    writes: Vec<ResourceId>,
}

impl Accessor for Dependencies {
    fn try_new() -> Option<Self> {
        // there's no default for this
        None
    }

    fn reads(&self) -> Vec<ResourceId> {
        let mut reads = self.reads.clone();
        reads.push(ResourceId::new::<ReflectionTable>());

        reads
    }

    fn writes(&self) -> Vec<ResourceId> {
        self.writes.clone()
    }
}

/// A dynamic system that represents and calls the script.
struct DynamicSystem {
    dependencies: Dependencies,
    /// just a dummy, you would want an actual script handle here
    script: fn(ScriptInput),
}

impl<'a> System<'a> for DynamicSystem {
    type SystemData = ScriptSystemData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let meta = data.meta_table;
        let reads: Vec<&dyn Reflection> = data
            .reads
            .iter()
            .map(|resource| meta.get(&**resource).expect("Not registered in meta table"))
            .collect();

        let writes: Vec<&mut dyn Reflection> = data
            .writes
            .iter_mut()
            .map(|resource| {
                // For some reason this needs a type ascription, otherwise Rust will think it's
                // a `&mut (Reflaction + '_)` (as opposed to `&mut (Reflection + 'static)`. (Note this
                // isn't needed in newer rust version but fails on the MSRV of 1.59.0).
                let res: &mut dyn Reflection = meta.get_mut(&mut **resource).expect(
                    "Not registered in meta \
                     table",
                );
                res
            })
            .collect();

        let input = ScriptInput { reads, writes };

        // call the script with the input
        (self.script)(input);
    }

    fn accessor<'b>(&'b self) -> AccessorCow<'a, 'b, Self> {
        AccessorCow::Ref(&self.dependencies)
    }

    fn setup(&mut self, _res: &mut World) {
        // this could call a setup function of the script
    }
}

/// Some trait that all of your dynamic resources should implement.
/// This trait should be able to register / transfer it to the scripting
/// framework.
trait Reflection {
    fn call_method(&self, s: &str);
}

// necessary for `MetaTable`
unsafe impl<T> CastFrom<T> for dyn Reflection
where
    T: Reflection + 'static,
{
    fn cast(t: *mut T) -> *mut Self {
        t
    }
}

type ReflectionTable = MetaTable<dyn Reflection>;

/// Maps resource names to resource ids.
struct ResourceTable {
    map: HashMap<String, ResourceId>,
}

impl ResourceTable {
    fn new() -> Self {
        ResourceTable {
            map: HashMap::default(),
        }
    }

    fn register<T: Resource>(&mut self, name: &str) {
        self.map.insert(name.to_owned(), ResourceId::new::<T>());
    }

    fn get(&self, name: &str) -> ResourceId {
        self.map.get(name).cloned().unwrap()
    }
}

struct ScriptInput<'a> {
    reads: Vec<&'a dyn Reflection>,
    writes: Vec<&'a mut dyn Reflection>,
}

struct ScriptSystemData<'a> {
    meta_table: Read<'a, ReflectionTable>,
    reads: Vec<AtomicRef<'a, dyn Resource + 'static>>,
    writes: Vec<AtomicRefMut<'a, dyn Resource + 'static>>,
}

impl<'a> DynamicSystemData<'a> for ScriptSystemData<'a> {
    type Accessor = Dependencies;

    fn setup(_accessor: &Dependencies, _res: &mut World) {}

    fn fetch(access: &Dependencies, world: &'a World) -> Self {
        let reads = access
            .reads
            .iter()
            .map(|id| {
                let id = id.clone();
                // SAFETY: We don't expose mutable reference to the Box or swap it out.
                let res = unsafe { world.try_fetch_internal(id) };
                AtomicRef::map(
                    res.expect("bug: the requested resource does not exist")
                        .borrow(),
                    Box::as_ref,
                )
            })
            .collect();
        let writes = access
            .writes
            .iter()
            .map(|id| {
                let id = id.clone();
                // SAFETY: We don't expose mutable reference to the Box or swap it out.
                let res = unsafe { world.try_fetch_internal(id) };
                AtomicRefMut::map(
                    res.expect("bug: the requested resource does not exist")
                        .borrow_mut(),
                    Box::as_mut,
                )
            })
            .collect();

        ScriptSystemData {
            meta_table: SystemData::fetch(world),
            reads,
            writes,
        }
    }
}

fn create_script_sys(res: &World) -> DynamicSystem {
    // -- what we get from the script --
    fn script(input: ScriptInput) {
        input.reads[0].call_method("bar");
        input.writes[0].call_method("foo");
    }

    let reads = ["Bar"];
    let writes = ["Foo"];

    // -- how we create the system --
    let table = res.fetch::<ResourceTable>();

    DynamicSystem {
        dependencies: Dependencies {
            reads: reads.iter().map(|r| table.get(r)).collect(),
            writes: writes.iter().map(|r| table.get(r)).collect(),
        },
        // just pass the function pointer
        script,
    }
}

fn main() {
    /// Some resource
    #[derive(Debug, Default)]
    struct Foo;

    impl Reflection for Foo {
        fn call_method(&self, s: &str) {
            match s {
                "foo" => println!("Hello from Foo"),
                "bar" => println!("You gotta ask somebody else"),
                _ => panic!("The error handling of this example is non-ideal"),
            }
        }
    }

    /// Another resource
    #[derive(Debug, Default)]
    struct Bar;

    impl Reflection for Bar {
        fn call_method(&self, s: &str) {
            match s {
                "bar" => println!("Hello from Bar"),
                "foo" => println!("You gotta ask somebody else"),
                _ => panic!("The error handling of this example is non-ideal"),
            }
        }
    }

    struct NormalSys;

    impl<'a> System<'a> for NormalSys {
        type SystemData = (Read<'a, Foo>, Read<'a, Bar>);

        fn run(&mut self, (foo, bar): Self::SystemData) {
            println!("Fetched foo: {:?}", &foo as &Foo);
            println!("Fetched bar: {:?}", &bar as &Bar);
        }
    }

    let mut res = World::empty();

    {
        let mut table = res.entry().or_insert_with(ReflectionTable::new);

        table.register::<Foo>();
        table.register::<Bar>();
    }

    {
        let mut table = res.entry().or_insert_with(ResourceTable::new);
        table.register::<Foo>("Foo");
        table.register::<Bar>("Bar");
    }

    let mut dispatcher = DispatcherBuilder::new()
        .with(NormalSys, "normal", &[])
        .build();
    dispatcher.setup(&mut res);

    let script0 = create_script_sys(&res);

    // it is recommended you create a second dispatcher dedicated to scripts,
    // that'll allow you to rebuild if necessary
    let mut scripts = DispatcherBuilder::new()
        .with(script0, "script0", &[])
        .build();
    scripts.setup(&mut res);

    // Game loop
    loop {
        dispatcher.dispatch(&res);
        scripts.dispatch(&res);
    }
}
