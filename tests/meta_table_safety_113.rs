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
    _pointer: Box<u64>,
}

unsafe impl CastFrom<MultipleData> for dyn PointsToU64 {
    fn cast(_t: *mut MultipleData) -> *mut Self {
        // This is wrong and will cause a panic
        //
        // NOTE: we use this instead of constructing a pointer to the field since
        // there is no way to easily and safely do that currently! (this can be
        // changed if offset_of macro is added to std).
        core::ptr::NonNull::<Box<u64>>::dangling().as_ptr()
    }
}

#[test]
#[should_panic(expected = "Bug: `CastFrom` did not cast `self`")]
fn test_panics() {
    let mut table: MetaTable<dyn PointsToU64> = MetaTable::new();
    let md = MultipleData {
        _number: 0x0, // this will be casted to a pointer, then dereferenced
        _pointer: Box::new(42),
    };
    table.register::<MultipleData>();
    if let Some(t) = table.get(&md) {
        println!("{}", t.get_u64());
    }
}
