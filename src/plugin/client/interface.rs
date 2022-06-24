use crate::common::*;
use bevy::prelude::Entity;

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
            Box::new(message::EntityUpdate {
                destroyed,
                entities: entities.map(|(e, n)| (e.into(), n)).collect(),
            }),
        )?;

        self.close(id)
    }
}
