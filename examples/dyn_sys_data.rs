//! Warning: this example is a lot more advanced than the others.
//!
//! This example shows a possible way to make shred interact with a scripting language.
//!
//! It does that by implementing `DynamicSystemData` and using `MetaTable`.

extern crate shred;

// in a real application you would use `fnv`
use std::collections::HashMap;

use shred::{Accessor, AccessorCow, CastFrom, DispatcherBuilder, DynamicSystemData, MetaTable, Read, Resource,
            ResourceId, Resources,
            System, SystemData, SystemFetch};
use shred::cell::{Ref, RefMut};

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

impl System for DynamicSystem {
    type SystemData = ScriptSystemData;

    fn run(&mut self, mut data: Self::SystemData) {
        let meta = data.meta_table;
        let reads: Vec<&Reflection> = data.reads.iter().map(|resource| {
            // explicitly use the type because we're dealing with `&Resource` which is implemented
            // by a lot of types; we don't want to accidentally get a `&Box<Resource>` and cast
            // it to a `&Resource`.
            let res = Box::as_ref(resource);

            meta.get(res).expect("Not registered in meta table")
        }).collect();

        let writes: Vec<&mut Reflection> = data.writes.iter_mut().map(|resource| {
            // explicitly use the type because we're dealing with `&mut Resource` which is
            // implemented by a lot of types; we don't want to accidentally get a
            // `&mut Box<Resource>` and cast it to a `&mut Resource`.
            let res = Box::as_mut(resource);

            // For some reason this needs a type ascription, otherwise Rust will think it's
            // a `&mut (Reflection + '_)` (as opposed to `&mut (Reflection + 'static)`.
            let res: &mut Reflection = meta.get_mut(res).expect("Not registered in meta table");

            res
        }).collect();

        let input = ScriptInput {
            reads,
            writes,
        };

        // call the script with the input
        (self.script)(input);
    }

    fn accessor<'b>(&'b self) -> AccessorCow<'b, Self> {
        AccessorCow::Ref(&self.dependencies)
    }

    fn setup(&mut self, _res: &mut Resources) {
        // this could call a setup function of the script
    }
}

/// Some trait that all of your dynamic resources should implement.
/// This trait should be able to register / transfer it to the scripting framework.
trait Reflection {
    fn call_method(&self, s: &str);
}

// necessary for `MetaTable`
impl<T> CastFrom<T> for Reflection
where
    T: Reflection + 'static,
{
    fn cast(t: &T) -> &Self {
        t
    }

    fn cast_mut(t: &mut T) -> &mut Self {
        t
    }
}

type ReflectionTable = MetaTable<Reflection>;

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
        *self.map.get(name).unwrap()
    }
}

struct ScriptInput<'a> {
    reads: Vec<&'a Reflection>,
    writes: Vec<&'a mut Reflection>,
}

struct ScriptSystemData {
    meta_table: Read<ReflectionTable>,
    reads: Vec<Ref<Box<Resource>>>,
    writes: Vec<RefMut<Box<Resource>>>,
}

impl DynamicSystemData for ScriptSystemData {
    type Accessor = Dependencies;

    fn setup(_accessor: &Dependencies, _res: &mut Resources) {}

    fn fetch<'r>(access: &Dependencies, res: &'r Resources) -> SystemFetch<'r, Self> {
        let reads = access
            .reads
            .iter()
            .map(|id| id.0)
            .map(|id| res
                .try_fetch_internal(id)
                .expect("bug: the requested resource does not exist")
                .borrow()
            )
            .collect();
        let writes = access
            .writes
            .iter()
            .map(|id| id.0)
            .map(|id| res
                .try_fetch_internal(id)
                .expect("bug: the requested resource does not exist")
                .borrow_mut()
            )
            .collect();

        SystemFetch::from(ScriptSystemData {
            meta_table: SystemData::fetch(res).into_inner(),
            reads,
            writes,
        })
    }
}

fn create_script_sys(res: &Resources) -> DynamicSystem {
    // -- what we get from the script --
    fn script(input: ScriptInput) {
        input.reads[0].call_method("bar");
        input.writes[0].call_method("foo");
    }

    let reads = vec!["Bar"];
    let writes = vec!["Foo"];

    // -- how we create the system --
    let table = res.fetch::<ResourceTable>();

    let sys = DynamicSystem {
        dependencies: Dependencies {
            reads: reads.iter().map(|r| table.get(r)).collect(),
            writes: writes.iter().map(|r| table.get(r)).collect(),
        },
        // just pass the function pointer
        script,
    };

    sys
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

    impl System for NormalSys {
        type SystemData = (Read<Foo>, Read<Bar>);

        fn run(&mut self, (foo, bar): Self::SystemData) {
            println!("Fetched foo: {:?}", &foo as &Foo);
            println!("Fetched bar: {:?}", &bar as &Bar);
        }
    }

    let mut res = Resources::new();

    {
        let mut table = res.entry().or_insert_with(|| ReflectionTable::new());

        table.register(&Foo);
        table.register(&Bar);
    }

    {
        let mut table = res.entry().or_insert_with(|| ResourceTable::new());
        table.register::<Foo>("Foo");
        table.register::<Bar>("Bar");
    }

    let mut dispatcher = DispatcherBuilder::new().with(NormalSys, "normal", &[]).build();
    dispatcher.setup(&mut res);

    let script0 = create_script_sys(&res);

    // it is recommended you create a second dispatcher dedicated to scripts,
    // that'll allow you to rebuild if necessary
    let mut scripts = DispatcherBuilder::new().with(script0, "script0", &[]).build();
    scripts.setup(&mut res);

    // Game loop
    loop {
        dispatcher.dispatch(&res);
        scripts.dispatch(&res);

        break;
    }
}
