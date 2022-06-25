use bevy_editor_iris_common::bevy::prelude::Entity;
use bevy_editor_iris_common::message::EntityUpdate;
use bevy_editor_iris_common::{Interface, InterfaceError};

pub trait ClientInterfaceExt {
    fn send_entity_update(
        &self,
        entities: impl Iterator<Item = (Entity, Option<String>)>,
        destroyed: bool,
    ) -> Result<(), InterfaceError>;
}

impl ClientInterfaceExt for Interface {
    fn send_entity_update(
        &self,
        entities: impl Iterator<Item = (Entity, Option<String>)>,
        destroyed: bool,
    ) -> Result<(), InterfaceError> {
        let id = self.send(
            None,
            Box::new(EntityUpdate {
                destroyed,
                entities: entities.map(|(e, n)| (e.into(), n)).collect(),
            }),
        )?;

        self.close(id)
    }
}
