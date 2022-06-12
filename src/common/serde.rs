use bevy::{prelude::*, reflect::TypeRegistry};
use serde::{de::DeserializeSeed, Deserialize, Serialize, Serializer};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Reflect)]
pub struct RemoteEntity {
    generation: u32,
    id: u32,
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
    TYPE_REGISTRY.with(|cell| cell.borrow_mut().take())
}

#[derive(Debug, Deref, DerefMut)]
pub struct ReflectObject(Box<dyn Reflect>);

impl From<Box<dyn Reflect>> for ReflectObject {
    fn from(b: Box<dyn Reflect>) -> Self {
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

            bevy::reflect::serde::ReflectSerializer::new(self.as_ref(), &lock).serialize(serializer)
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
                .map(|b| b.into())
        })
    }
}
