use crate::common::{
    ClientMessage, EditorMessage, Interface, ReflectObject, RemoteEntity, WaitForResponseError,
};
use bevy::prelude::*;
use std::any::type_name;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

pub type ClientInterface = Interface<ClientMessage, EditorMessage>;

impl ClientInterface {
    pub fn collect_entity_updates(&mut self) -> Vec<Vec<RemoteEntity>> {
        self.get_unhandled_responses(|msg| matches!(msg, ClientMessage::EntityUpdate(..)))
            .into_iter()
            .map(|msg| match msg {
                ClientMessage::EntityUpdate(entities) => entities,
                _ => unreachable!(),
            })
            .collect()
    }

    pub fn query_component<T: Component>(
        &mut self,
        entities: Vec<RemoteEntity>,
    ) -> Result<Vec<ReflectObject>, QueryComponentError> {
        self.outgoing.blocking_send(EditorMessage::ComponentQuery(
            vec![type_name::<T>().to_string()],
            entities,
        ))?;

        Ok(match self
            .wait_for_response(|msg| matches!(msg, ClientMessage::ComponentResponse(..)))?
        {
            ClientMessage::ComponentResponse(components) => components,
            _ => unreachable!(),
        }
        .into_iter()
        .flatten()
        .collect())
    }

    pub fn query_components<T: ComponentQuery>(
        &mut self,
        entities: Vec<RemoteEntity>,
    ) -> Result<Vec<Vec<ReflectObject>>, QueryComponentError> {
        self.outgoing
            .blocking_send(EditorMessage::ComponentQuery(T::into_names(), entities))?;

        Ok(
            match self
                .wait_for_response(|msg| matches!(msg, ClientMessage::ComponentResponse(..)))?
            {
                ClientMessage::ComponentResponse(components) => components,
                _ => unreachable!(),
            },
        )
    }
}

#[derive(Debug, Error)]
pub enum QueryComponentError {
    #[error("encountered a WaitForResponseError")]
    WaitForResponseError(#[from] WaitForResponseError),
    #[error("encountered a SendError")]
    SendError(#[from] SendError<EditorMessage>),
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
