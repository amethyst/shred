extern crate shred;

use shred::{CastFrom, MetaTable};

pub trait PointsToU64 {
    fn get_u64(&self) -> u64;
}

impl PointsToU64 for Box<u64> {
    fn get_u64(&self) -> u64 {
        **self
    }
}

struct MultipleData {
    _number: u64,
    pointer: Box<u64>,
}

unsafe impl CastFrom<MultipleData> for dyn PointsToU64 {
    fn cast(t: &MultipleData) -> &Self {
        // this is wrong and will cause a panic
        &t.pointer
    }

    fn cast_mut(t: &mut MultipleData) -> &mut Self {
        &mut t.pointer
    }
}

#[test]
#[should_panic(expected = "Bug: `CastFrom` did not cast `self`")]
fn test_panics() {
    let mut table: MetaTable<dyn PointsToU64> = MetaTable::new();
    let md = MultipleData {
        _number: 0x0, // this will be casted to a pointer, then dereferenced
        pointer: Box::new(42),
    };
    table.register(&md);
    if let Some(t) = table.get(&md) {
        println!("{}", t.get_u64());
    }
}
