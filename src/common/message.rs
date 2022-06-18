use std::any::Any;

use super::serde::{ReflectObject, RemoteEntity};
use bevy::{
    prelude::*,
    reflect::{FromReflect, FromType, TypeRegistry},
};
use bevy_mod_ouroboros_derive::*;

// #[non_exhaustive]
// #[derive(Debug, Deserialize, Serialize)]
// pub enum EditorMessage {
//     ComponentQuery(Vec<String>, Vec<RemoteEntity>),
//     Ping,
// }

pub trait EditorMessage: Reflect {
    fn any(self: Box<Self>) -> Box<dyn Any>;

    fn any_ref(&self) -> &dyn Any;

    fn any_mut(&mut self) -> &mut dyn Any;

    fn reflect(self: Box<Self>) -> Box<dyn Reflect>;

    fn borrow_reflect(&self) -> &dyn Reflect;

    fn borrow_reflect_mut(&mut self) -> &mut dyn Reflect;
}

impl std::fmt::Debug for dyn EditorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.any_ref().fmt(f)
    }
}

// TODO: This can be replaced entirely by #[reflect_trait] in bevy 0.8, I just need the `get_boxed` method which is absent in 0.7
#[derive(Clone)]
pub struct ReflectEditorMessage {
    get_func: fn(&dyn Reflect) -> Option<&dyn EditorMessage>,
    get_mut_func: fn(&mut dyn Reflect) -> Option<&mut dyn EditorMessage>,
    get_boxed_func: fn(Box<dyn Reflect>) -> Result<Box<dyn EditorMessage>, Box<dyn Reflect>>,
}

impl ReflectEditorMessage {
    pub fn get<'a>(&self, reflect_value: &'a dyn Reflect) -> Option<&'a dyn EditorMessage> {
        (self.get_func)(reflect_value)
    }

    pub fn get_mut<'a>(
        &self,
        reflect_value: &'a mut dyn Reflect,
    ) -> Option<&'a mut dyn EditorMessage> {
        (self.get_mut_func)(reflect_value)
    }

    pub fn get_boxed(
        &self,
        reflect_value: Box<dyn Reflect>,
    ) -> Result<Box<dyn EditorMessage>, Box<dyn Reflect>> {
        (self.get_boxed_func)(reflect_value)
    }
}

impl<T: EditorMessage + Reflect> FromType<T> for ReflectEditorMessage {
    fn from_type() -> Self {
        Self {
            get_func: |reflect_value| {
                reflect_value
                    .downcast_ref::<T>()
                    .map(|value| value as &dyn EditorMessage)
            },
            get_mut_func: |reflect_value| {
                reflect_value
                    .downcast_mut::<T>()
                    .map(|value| value as &mut dyn EditorMessage)
            },
            get_boxed_func: |reflect_value| {
                reflect_value
                    .downcast::<T>()
                    .map(|value| value as Box<dyn EditorMessage>)
            },
        }
    }
}

// #[non_exhaustive]
// #[derive(Debug, Deserialize, Serialize)]
// pub enum ClientMessage {
//     ComponentResponse(Vec<Vec<ReflectObject>>),
//     EntityUpdate(Vec<RemoteEntity>),
//     Ping,
// }

pub trait ClientMessage: Reflect {
    fn any(self: Box<Self>) -> Box<dyn Any>;

    fn any_ref(&self) -> &dyn Any;

    fn any_mut(&mut self) -> &mut dyn Any;

    fn reflect(self: Box<Self>) -> Box<dyn Reflect>;

    fn borrow_reflect(&self) -> &dyn Reflect;

    fn borrow_reflect_mut(&mut self) -> &mut dyn Reflect;
}

impl dyn ClientMessage {
    /// Returns `true` if the underlying value is of type `T`, or `false`
    /// otherwise.
    #[inline]
    pub fn is<T: 'static>(&self) -> bool {
        self.any_ref().is::<T>()
    }
}

impl std::fmt::Debug for dyn ClientMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.any_ref().fmt(f)
    }
}

// TODO: This can be replaced entirely by #[reflect_trait] in bevy 0.8, I just need the `get_boxed` method which is absent in 0.7
#[derive(Clone)]
pub struct ReflectClientMessage {
    get_func: fn(&dyn Reflect) -> Option<&dyn ClientMessage>,
    get_mut_func: fn(&mut dyn Reflect) -> Option<&mut dyn ClientMessage>,
    get_boxed_func: fn(Box<dyn Reflect>) -> Result<Box<dyn ClientMessage>, Box<dyn Reflect>>,
}

impl ReflectClientMessage {
    pub fn get<'a>(&self, reflect_value: &'a dyn Reflect) -> Option<&'a dyn ClientMessage> {
        (self.get_func)(reflect_value)
    }

    pub fn get_mut<'a>(
        &self,
        reflect_value: &'a mut dyn Reflect,
    ) -> Option<&'a mut dyn ClientMessage> {
        (self.get_mut_func)(reflect_value)
    }

    pub fn get_boxed(
        &self,
        reflect_value: Box<dyn Reflect>,
    ) -> Result<Box<dyn ClientMessage>, Box<dyn Reflect>> {
        (self.get_boxed_func)(reflect_value)
    }
}

impl<T: ClientMessage + Reflect> FromType<T> for ReflectClientMessage {
    fn from_type() -> Self {
        Self {
            get_func: |reflect_value| {
                reflect_value
                    .downcast_ref::<T>()
                    .map(|value| value as &dyn ClientMessage)
            },
            get_mut_func: |reflect_value| {
                reflect_value
                    .downcast_mut::<T>()
                    .map(|value| value as &mut dyn ClientMessage)
            },
            get_boxed_func: |reflect_value| {
                reflect_value
                    .downcast::<T>()
                    .map(|value| value as Box<dyn ClientMessage>)
            },
        }
    }
}

// impl Default for ClientMessage {
//     fn default() -> Self {
//         ClientMessage::Ping
//     }
// }

#[dual_message]
pub struct Ping;

#[client_message]
pub struct EntityUpdate {
    pub entities: Vec<RemoteEntity>,
}

#[derive(Default)]
#[client_message]
pub struct ComponentResponse {
    pub components: Vec<Vec<ReflectObject>>,
}

#[derive(Default)]
#[editor_message]
pub struct ComponentQuery {
    pub components: Vec<String>,
    pub entities: Vec<RemoteEntity>,
}

pub fn register_messages(registry: ResMut<TypeRegistry>) {
    let mut registry = registry.write();

    macro_rules! register {
        ($($ty:path),* $(,)?) => {
            $(
                registry.register::<$ty>();
            )*
        }
    }

    register![Ping, EntityUpdate, ComponentResponse,];
}

// TODO: This may be replaced in the future by something like `ReflectFromReflect`, but unfortunately for now this is necessary
pub trait MessageFromReflect {
    fn from_reflect(&self, reflect: &dyn Reflect) -> Option<Box<Self>>;
}

impl<T: FromReflect> MessageFromReflect for T {
    fn from_reflect(&self, reflect: &dyn Reflect) -> Option<Box<Self>> {
        <T as FromReflect>::from_reflect(reflect).map(|this| Box::new(this))
    }
}

#[derive(Clone)]
pub struct ReflectMessageFromReflect {
    from_reflect: fn(&dyn Reflect) -> Option<Box<dyn Reflect>>,
}

impl ReflectMessageFromReflect {
    pub fn from_reflect(&self, reflect: &dyn Reflect) -> Option<Box<dyn Reflect>> {
        (self.from_reflect)(reflect)
    }
}

impl<T: Reflect + FromReflect> FromType<T> for ReflectMessageFromReflect {
    fn from_type() -> Self {
        Self {
            from_reflect: |reflect| {
                <T as FromReflect>::from_reflect(reflect)
                    .map(|val| Box::new(val) as Box<dyn Reflect>)
            },
        }
    }
}

#[test]
fn component_response_ser() -> Result<(), Box<dyn std::error::Error>> {
    let registry = TypeRegistry::default();

    {
        let mut registry = registry.write();
        registry.register::<i32>();
        registry.register::<u32>();
        registry.register::<usize>();
        registry.register::<String>();
        registry.register::<Vec4>();
        registry.register::<ComponentResponse>();
        registry.register::<Vec<Vec<ReflectObject>>>();
    }

    let _ = crate::common::replace_type_registry(registry);

    let msg = ComponentResponse {
        components: vec![
            vec![Box::new(12i32).into(), Box::new("bruh".to_string()).into()],
            vec![
                Box::new(72u32).into(),
                Box::new(100usize).into(),
                Box::new("SHITE".to_string()).into(),
            ],
        ],
    };

    let object: ReflectObject = Box::new(msg).into();

    let ser = serde_yaml::to_string(&object)?;

    println!("{ser}");

    let deser: ReflectObject = serde_yaml::from_str(&ser)?;
    let reflect: Box<dyn Reflect> = deser.into();
    use std::ops::Deref;
    let x = reflect.type_name();
    let y = reflect.deref().type_name();

    println!("{x}");
    println!("{y}");

    let mut deser: ComponentResponse = ComponentResponse::default();
    deser.apply(reflect.as_ref());

    Ok(())
}
