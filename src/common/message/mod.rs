use bevy::{
    prelude::*,
    reflect::{FromReflect, FromType},
};
use std::any::Any;
use thiserror::Error;

mod distributor;
mod messages;

pub use distributor::*;
pub use messages::*;

// TODO: This may end up in bevy alongside `Reflect`
pub trait IntoAny {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any(&self) -> &dyn Any;

    fn as_mut_any(&mut self) -> &mut dyn Any;
}

impl<T: Any> IntoAny for T {
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

// TODO: This may end up in bevy alongside `Reflect`
pub trait IntoReflect: Reflect {
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect>;

    fn as_reflect(&self) -> &dyn Reflect;

    fn as_mut_reflect(&mut self) -> &mut dyn Reflect;
}

impl<T: Reflect> IntoReflect for T {
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    fn as_mut_reflect(&mut self) -> &mut dyn Reflect {
        self
    }
}

pub trait Message: Reflect + IntoAny + IntoReflect {}

impl std::fmt::Debug for dyn Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_reflect().fmt(f)
    }
}

// TODO: This can be replaced entirely by #[reflect_trait] in bevy 0.8, I just need the `get_boxed` method which is absent in 0.7
#[derive(Clone)]
pub struct ReflectMessage {
    get_func: fn(&dyn Reflect) -> Option<&dyn Message>,
    get_mut_func: fn(&mut dyn Reflect) -> Option<&mut dyn Message>,
    get_boxed_func: fn(Box<dyn Reflect>) -> Result<Box<dyn Message>, Box<dyn Reflect>>,
}

impl ReflectMessage {
    pub fn get<'a>(&self, reflect_value: &'a dyn Reflect) -> Option<&'a dyn Message> {
        (self.get_func)(reflect_value)
    }

    pub fn get_mut<'a>(&self, reflect_value: &'a mut dyn Reflect) -> Option<&'a mut dyn Message> {
        (self.get_mut_func)(reflect_value)
    }

    pub fn get_boxed(
        &self,
        reflect_value: Box<dyn Reflect>,
    ) -> Result<Box<dyn Message>, Box<dyn Reflect>> {
        (self.get_boxed_func)(reflect_value)
    }
}

impl<T: Message + Reflect> FromType<T> for ReflectMessage {
    fn from_type() -> Self {
        Self {
            get_func: |reflect_value| {
                reflect_value
                    .downcast_ref::<T>()
                    .map(|value| value as &dyn Message)
            },
            get_mut_func: |reflect_value| {
                reflect_value
                    .downcast_mut::<T>()
                    .map(|value| value as &mut dyn Message)
            },
            get_boxed_func: |reflect_value| {
                reflect_value
                    .downcast::<T>()
                    .map(|value| value as Box<dyn Message>)
            },
        }
    }
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

pub fn serialize_message<M: ?Sized + Message>(msg: Box<M>) -> serde_yaml::Result<Vec<u8>> {
    crate::common::with_type_registry(|reg| {
        let reg = reg.unwrap().read();

        let refl = bevy::reflect::serde::ReflectSerializer::new(msg.as_reflect(), &*reg);

        serde_yaml::to_vec(&refl)
    })
}

#[derive(Debug, Error)]
pub enum MessageDeserError {
    #[error(transparent)]
    YamlError(#[from] serde_yaml::Error),
    #[error("the received message {} is not registered in the TypeRegistry", .0)]
    MessageNotRegistered(String),
    #[error("the received message {} does not have an accessible FromReflect implementation; make sure to use #[reflect(MessageFromReflect)]", .0)]
    MessageNotFromReflect(String),
    #[error("the received message could not be converted to a concrete type: {:#?}", .0)]
    FromReflectFailed(String),
    #[error("the received message {} does not have an accessible Message implementation; make sure to use #[reflect(Message)] or #[message]", .0)]
    MessageNotImpl(String),
}

pub fn deserialize_message(buf: &[u8]) -> Result<Box<dyn Message>, MessageDeserError> {
    crate::common::with_type_registry(|reg| {
        let reg = reg.unwrap().read();

        let deser = bevy::reflect::serde::ReflectDeserializer::new(&reg);

        let dynamic = serde_yaml::seed::from_slice_seed(buf, deser)?;

        let registration = reg
            .get_with_name(dynamic.type_name())
            .ok_or_else(|| MessageDeserError::MessageNotRegistered(dynamic.type_name().into()))?;

        let from_reflect = registration
            .data::<ReflectMessageFromReflect>()
            .ok_or_else(|| MessageDeserError::MessageNotFromReflect(dynamic.type_name().into()))?;

        let msg = from_reflect.from_reflect(&*dynamic).ok_or_else(|| {
            MessageDeserError::FromReflectFailed(String::from_utf8_lossy(&buf).to_string())
        })?;

        let reflect_msg = registration
            .data::<ReflectMessage>()
            .ok_or_else(|| MessageDeserError::MessageNotImpl(dynamic.type_name().into()))?;

        let msg = reflect_msg.get_boxed(msg).unwrap();

        // Type inference died here, not sure why this is necessary
        Ok::<_, MessageDeserError>(msg)
    })
}
