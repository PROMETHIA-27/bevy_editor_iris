use crate::common::*;
use bevy::prelude::*;
use std::any::type_name;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

pub type ClientInterface = Interface<Box<dyn ClientMessage>, Box<dyn EditorMessage>>;

impl ClientInterface {
    pub fn collect_messages<M: ClientMessage>(&mut self) -> Vec<M> {
        self.get_unhandled_responses(|msg| msg.is::<M>())
            .into_iter()
            .map(|msg| *msg.any().downcast::<M>().unwrap())
            .collect()
    }

    pub fn query_component<T: Component>(
        &mut self,
        entities: Vec<RemoteEntity>,
    ) -> Result<Vec<ReflectObject>, QueryComponentError> {
        self.outgoing
            .blocking_send(Box::new(message::ComponentQuery {
                components: vec![type_name::<T>().to_string()],
                entities,
            }))?;

        Ok(self
            .wait_for_response(|msg| msg.is::<message::ComponentResponse>())?
            .any()
            .downcast::<message::ComponentResponse>()
            .unwrap()
            .components
            .into_iter()
            .flatten()
            .collect())
    }

    pub fn query_components<T: ComponentQuery>(
        &mut self,
        entities: Vec<RemoteEntity>,
    ) -> Result<Vec<Vec<ReflectObject>>, QueryComponentError> {
        self.outgoing
            .blocking_send(Box::new(message::ComponentQuery {
                components: T::into_names(),
                entities,
            }))?;

        Ok(self
            .wait_for_response(|msg| msg.is::<message::ComponentResponse>())?
            .any()
            .downcast::<message::ComponentResponse>()
            .unwrap()
            .components
            .into_iter()
            .collect())
    }
}

#[derive(Debug, Error)]
pub enum QueryComponentError {
    #[error(transparent)]
    WaitForResponseError(#[from] WaitForResponseError),
    #[error(transparent)]
    SendError(#[from] SendError<Box<dyn EditorMessage>>),
}

pub trait ComponentQuery {
    fn into_names() -> Vec<String>;
}

macro_rules! impl_component_query {
    ($($ts:tt),+) => {
        impl<$($ts: Component),+> ComponentQuery for ($($ts,)+) {
            fn into_names() -> Vec<String> {
                vec![$(type_name::<$ts>().to_string(),)+]
            }
        }
    };
}

impl_component_query!(T1);
impl_component_query!(T1, T2);
impl_component_query!(T1, T2, T3);
impl_component_query!(T1, T2, T3, T4);
impl_component_query!(T1, T2, T3, T4, T5);
impl_component_query!(T1, T2, T3, T4, T5, T6);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_component_query!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
