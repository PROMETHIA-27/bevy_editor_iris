use std::cell::RefCell;
use std::hash::Hash;

use bevy::prelude::{default, Entity};
use bevy::reflect::{FromReflect, Reflect, TypeRegistry};
use serde::de::DeserializeSeed;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Copy, Debug, Deserialize, Eq, FromReflect, Hash, PartialEq, Reflect, Serialize)]
#[reflect(Hash, PartialEq)]
pub struct RemoteEntity {
    pub(crate) generation: u32,
    pub(crate) id: u32,
}

impl RemoteEntity {
    pub fn to_string(&self) -> String {
        format!("{}v{}", self.id, self.generation)
    }
}

impl From<Entity> for RemoteEntity {
    fn from(entity: Entity) -> Self {
        RemoteEntity {
            generation: entity.generation(),
            id: entity.id(),
        }
    }
}

// TODO: There should be a built-in solution to this in bevy in the future. 6/10/2022
thread_local!(static TYPE_REGISTRY: RefCell<Option<TypeRegistry>> = default());

pub fn with_type_registry_context<F: FnOnce() -> R, R>(
    registry: TypeRegistry,
    f: F,
) -> (TypeRegistry, R) {
    let old_reg = replace_type_registry(registry);

    let r = f();

    let registry = match old_reg {
        Some(old) => {
            replace_type_registry(old).expect("Type registry unexpectedly removed from TLS")
        }
        None => take_type_registry().expect("Type registry unexpectedly removed from TLS"),
    };

    (registry, r)
}

pub fn with_type_registry<F: FnOnce(Option<&TypeRegistry>) -> R, R>(f: F) -> R {
    TYPE_REGISTRY.with(|registry| f(registry.borrow().as_ref()))
}

pub fn replace_type_registry(registry: TypeRegistry) -> Option<TypeRegistry> {
    TYPE_REGISTRY.with(|cell| cell.borrow_mut().replace(registry))
}

pub fn take_type_registry() -> Option<TypeRegistry> {
    TYPE_REGISTRY.with(|cell| cell.take())
}

// TODO: There should be a native solution to this in bevy in the future, and ReflectObject can be entirely removed.
// 6/17/2022
#[derive(Debug)]
pub struct ReflectObject(Box<dyn Reflect>);

unsafe impl Reflect for ReflectObject {
    fn type_name(&self) -> &str {
        self.0.type_name()
    }

    // Avoid UB which is caused by not returning self
    // This does lead to unintuitive behavior, though.
    fn any(&self) -> &dyn std::any::Any {
        self
    }

    fn any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn apply(&mut self, value: &dyn Reflect) {
        self.0.apply(value)
    }

    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        self.0.set(value)
    }

    fn reflect_ref(&self) -> bevy::reflect::ReflectRef {
        self.0.reflect_ref()
    }

    fn reflect_mut(&mut self) -> bevy::reflect::ReflectMut {
        self.0.reflect_mut()
    }

    fn clone_value(&self) -> Box<dyn Reflect> {
        self.0.clone_value()
    }

    fn reflect_hash(&self) -> Option<u64> {
        self.0.reflect_hash()
    }

    fn reflect_partial_eq(&self, value: &dyn Reflect) -> Option<bool> {
        self.0.reflect_partial_eq(value)
    }

    fn serializable(&self) -> Option<bevy::reflect::serde::Serializable> {
        Some(bevy::reflect::serde::Serializable::Borrowed(self))
    }
}

impl FromReflect for ReflectObject {
    fn from_reflect(reflect: &dyn Reflect) -> Option<Self> {
        Some(Self(reflect.clone_value()))
    }
}

impl From<Box<dyn Reflect>> for ReflectObject {
    fn from(b: Box<dyn Reflect>) -> Self {
        Self(b)
    }
}

impl<T: Reflect> From<Box<T>> for ReflectObject {
    fn from(b: Box<T>) -> Self {
        let b: Box<dyn Reflect> = b;
        Self(b)
    }
}

impl From<ReflectObject> for Box<dyn Reflect> {
    fn from(r: ReflectObject) -> Self {
        r.0
    }
}

impl Serialize for ReflectObject {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        with_type_registry(|reg| {
            let reg = reg.expect("Type registry must be placed in TLS to perform serialization");
            let lock = reg.internal.read();

            bevy::reflect::serde::ReflectSerializer::new(self.0.as_ref(), &lock)
                .serialize(serializer)
        })
    }
}

impl<'de> Deserialize<'de> for ReflectObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        with_type_registry(|reg| {
            let reg = reg.expect("Type registry must be placed in TLS to perform serialization");
            let lock = reg.internal.read();

            bevy::reflect::serde::ReflectDeserializer::new(&lock)
                .deserialize(deserializer)
                .map(|b| Self(b))
        })
    }
}

#[test]
fn reflect_object_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let registry = TypeRegistry::default();

    #[derive(Clone, Debug, Default, PartialEq, Reflect)]
    struct TestStruct {
        x: i32,
        str: String,
        vec: Vec4,
    }

    let test = TestStruct {
        x: 12,
        str: "Test".to_string(),
        vec: Vec4::new(1.0, 2.0, 3.0, 4.0),
    };

    {
        let mut registry = registry.write();
        registry.register::<i32>();
        registry.register::<String>();
        registry.register::<Vec4>();
        registry.register::<TestStruct>();
    }

    let _ = replace_type_registry(registry);

    let reflect: Box<dyn Reflect> = Box::new(test.clone());

    let object: ReflectObject = reflect.into();

    let ser = serde_yaml::to_string(&object)?;

    println!("{ser}");

    let deser: ReflectObject = serde_yaml::from_str(&ser)?;
    let reflect: Box<dyn Reflect> = deser.into();
    let mut deser_test: TestStruct = TestStruct::default();
    deser_test.apply(reflect.as_ref());

    assert_eq!(test, deser_test);

    Ok(())
}
