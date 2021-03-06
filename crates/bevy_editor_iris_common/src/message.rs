use std::any::Any;
use std::io;

use bevy::reflect::{FromReflect, FromType, Reflect};

use crate::error::MessageDeserError;
use crate::serde;

// TODO: This may end up in bevy alongside `Reflect`
/// Blanket impl to cast a type to [`Any`].
pub trait IntoAny: Any {
    /// Consumes a [boxed](Box) type and casts it to `dyn Any`.
    fn into_any(self: Box<Self>) -> Box<dyn Any>;

    /// Casts a reference to `dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Casts a mutable reference to `dyn Any`.
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
/// Blanket impl to cast a type to [`Reflect`].
pub trait IntoReflect: Reflect {
    /// Consumes a [boxed](Box) type and casts it to `dyn Reflect`.
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect>;

    /// Casts a reference to `dyn Reflect`.
    fn as_reflect(&self) -> &dyn Reflect;

    /// Casts a mutable reference to `dyn Reflect`.
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

/// A trait that marks a type as being sendable as a message
/// to the remote application.
pub trait Message: Reflect + IntoAny + IntoReflect {}

impl dyn Message {
    /// Returns `true` if this message is of type `T` and `false` otherwise.
    pub fn is<T: Any>(&self) -> bool {
        self.as_any().is::<T>()
    }

    // TODO: Reference downcasts
    /// Consumes and converts this message into type `T` if this message
    /// is an instance of type `T`. Returns `Err(self)` otherwise.
    pub fn downcast<T: Any>(self: Box<Self>) -> Result<T, Box<Self>> {
        if self.is::<T>() {
            Ok(*self.into_any().downcast::<T>().unwrap())
        } else {
            Err(self)
        }
    }
}

impl std::fmt::Debug for dyn Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_reflect().fmt(f)
    }
}

// TODO: This can be replaced entirely by #[reflect_trait] in bevy 0.8, I just need the `get_boxed` method which is absent in 0.7
/// Allows casting a `dyn Reflect` to a `dyn Message`, if the `dyn Reflect` is the correct type.
#[derive(Clone)]
pub struct ReflectMessage {
    get_func: fn(&dyn Reflect) -> Option<&dyn Message>,
    get_mut_func: fn(&mut dyn Reflect) -> Option<&mut dyn Message>,
    get_boxed_func: fn(Box<dyn Reflect>) -> Result<Box<dyn Message>, Box<dyn Reflect>>,
}

impl ReflectMessage {
    /// Converts a `&dyn Reflect` to a `&dyn Message`
    pub fn get<'a>(&self, reflect_value: &'a dyn Reflect) -> Option<&'a dyn Message> {
        (self.get_func)(reflect_value)
    }

    /// Converts a `&mut dyn Reflect` to a `&mut dyn Message`
    pub fn get_mut<'a>(&self, reflect_value: &'a mut dyn Reflect) -> Option<&'a mut dyn Message> {
        (self.get_mut_func)(reflect_value)
    }

    /// Converts a `Box<dyn Reflect>` to a `Box<dyn Message>`
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
/// Mirror of FromReflect to be used by [`ReflectMessageFromReflect`]
pub trait MessageFromReflect {
    /// Mirror of [`FromReflect::from_reflect()`]
    fn from_reflect(&self, reflect: &dyn Reflect) -> Option<Box<Self>>;
}

impl<T: FromReflect> MessageFromReflect for T {
    fn from_reflect(&self, reflect: &dyn Reflect) -> Option<Box<Self>> {
        <T as FromReflect>::from_reflect(reflect).map(|this| Box::new(this))
    }
}

/// Contains the FromReflect implementation of a type. Used as a temporary stopgap while waiting for an official
/// `ReflectFromReflect` to be added to bevy.
#[derive(Clone)]
pub struct ReflectMessageFromReflect {
    from_reflect: fn(&dyn Reflect) -> Option<Box<dyn Reflect>>,
}

impl ReflectMessageFromReflect {
    /// See [`FromReflect`]
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

/// Attempt to serialize a message into a yaml byte writer.
pub fn serialize_message<M: ?Sized + Message>(
    msg: Box<M>,
    writer: impl io::Write,
) -> serde_yaml::Result<()> {
    serde::with_type_registry(|reg| {
        let reg = reg.unwrap().read();

        let refl = bevy::reflect::serde::ReflectSerializer::new(msg.as_reflect(), &*reg);

        serde_yaml::to_writer(writer, &refl)
    })
}

/// Attempt to deserialize a [`Message`] from a yaml byte slice
pub fn deserialize_message(buf: &[u8]) -> Result<Box<dyn Message>, MessageDeserError> {
    serde::with_type_registry(|reg| {
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
