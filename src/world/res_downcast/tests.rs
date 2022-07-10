use std::any::TypeId;

use crate::Resource;

pub struct MyResource {}
pub struct AnotherResource {}

#[test]
fn dyn_has_correct_type_id() {
    let my_resource = MyResource {};
    let my_resource_dyn: &dyn Resource = &my_resource;

    assert_eq!(my_resource_dyn.type_id(), TypeId::of::<MyResource>());
}

#[test]
fn downcast_allowed() {
    let my_resource = MyResource {};
    let my_resource_dyn: &dyn Resource = &my_resource;

    assert!(my_resource_dyn.downcast_ref::<MyResource>().is_some());
}

#[test]
fn downcast_disallowed() {
    let my_resource = MyResource {};
    let my_resource_dyn: &dyn Resource = &my_resource;

    assert!(my_resource_dyn.downcast_ref::<AnotherResource>().is_none());
}
