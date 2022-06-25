use bevy::ecs::archetype::ArchetypeId;
use bevy::ecs::component::{ComponentId, ComponentTicks, Components, StorageType};
use bevy::prelude::{App, Entity, Reflect, World};
use bevy::reflect::{FromReflect, TypeRegistry};
use bevy::utils::HashMap;
use bevy_mod_ouroboros_derive::{message, Message};

use crate::message::{Message, ReflectMessage, ReflectMessageFromReflect, RegisterMessage};
use crate::serde::{ReflectObject, RemoteEntity};

#[message]
pub struct Ping;

// TODO: Decide whether or not to split this into "NewEntities" and "DestroyedEntities"
// TODO: Should this send full entity data? Without that, it's difficult to actually request all component data
// to display in the inspector.
// TODO: This should be a `HashMap<RemoteEntity, Option<String>>`, but due to reflection serialization limitations
// it must not be a `Hash___` type
#[message]
pub struct EntityUpdate {
    pub destroyed: bool,
    pub entities: Vec<(RemoteEntity, Option<String>)>,
}

#[derive(Default)]
#[message]
pub struct ComponentResponse {
    pub components: HashMap<RemoteEntity, HashMap<String, ReflectObject>>,
}

#[derive(Default)]
#[message]
pub struct ComponentQuery {
    pub components: Vec<String>,
    pub entities: Vec<RemoteEntity>,
}

#[message]
pub struct CloseTransaction;

#[message]
pub struct SceneUpdate {
    // pub scene: DynamicScene,
}

pub fn what_changed(world: &mut World) {
    let mut map: HashMap<Entity, Vec<String>> = HashMap::default();

    let change_tick = world.change_tick();
    let registry = world.remove_resource::<TypeRegistry>().unwrap();
    let archetypes = world.archetypes();
    let storages = world.storages();
    let entities = world.entities();
    let components = world.components();
    let reg = registry.read();

    for archetype in archetypes.iter().filter(|archetype| match archetype.id() {
        ArchetypeId::EMPTY | ArchetypeId::RESOURCE | ArchetypeId::INVALID => false,
        _ => true,
    }) {
        let table_id = archetype.table_id();
        let table = storages.tables.get(table_id).unwrap();

        for component_id in archetype.components() {
            let storage_type = archetype.get_storage_type(component_id).unwrap();
            match storage_type {
                StorageType::Table => {
                    let column = table.get_column(component_id).unwrap();

                    for entity in archetype.entities() {
                        let location = entities.get(*entity).unwrap();
                        let table_row = archetype.entity_table_row(location.index);
                        // Safe: the table row is obtained safely from the world's state
                        let ticks = unsafe { column.get_ticks_unchecked(table_row) };

                        insert_changed(
                            ticks,
                            world,
                            change_tick,
                            components,
                            component_id,
                            &mut map,
                            entity,
                        );
                    }
                }
                StorageType::SparseSet => {
                    let sparse_set = storages.sparse_sets.get(component_id).unwrap();

                    for entity in archetype.entities() {
                        let ticks = sparse_set.get_ticks(*entity).unwrap();

                        insert_changed(
                            ticks,
                            world,
                            change_tick,
                            components,
                            component_id,
                            &mut map,
                            entity,
                        );
                    }
                }
            }
        }
    }

    println!("{map:#?}");

    drop(reg);
    world.insert_resource(registry);

    fn insert_changed(
        ticks: &ComponentTicks,
        world: &World,
        change_tick: u32,
        components: &Components,
        component_id: ComponentId,
        map: &mut HashMap<Entity, Vec<String>>,
        entity: &Entity,
    ) {
        // TODO: Need someone who knows more about bevy_ecs to review this
        if ticks.is_changed(world.last_change_tick(), change_tick) {
            // Safe: The component ID is valid
            let type_name = unsafe { components.get_info_unchecked(component_id) }
                .name()
                .into();

            match map.get_mut(entity) {
                Some(vec) => vec.push(type_name),
                None => _ = map.insert(*entity, vec![type_name]),
            }
        }
    }
}

pub struct DefaultMessages;

impl RegisterMessage for DefaultMessages {
    fn register(app: &mut App) {
        Ping::register(app);
        EntityUpdate::register(app);
        ComponentResponse::register(app);
        ComponentQuery::register(app);
        CloseTransaction::register(app);
    }
}
