use crate::common::*;
use bevy::prelude::Entity;

pub trait ClientInterfaceExt {
    fn send_entity_update(
        &self,
        entities: impl Iterator<Item = Entity>,
        destroyed: bool,
    ) -> Result<(), InterfaceError>;
}

impl ClientInterfaceExt for Interface {
    fn send_entity_update(
        &self,
        entities: impl Iterator<Item = Entity>,
        destroyed: bool,
    ) -> Result<(), InterfaceError> {
        let id = self.send(
            None,
            Box::new(message::EntityUpdate {
                destroyed,
                entities: entities.map(|e| e.into()).collect(),
            }),
        )?;

        self.close(id)
    }
}
