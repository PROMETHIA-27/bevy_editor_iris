use std::any::TypeId;
use std::sync::mpsc::{Receiver, Sender};

use ouroboros_common::asynchronous::{self, RemoteThreadError};
use ouroboros_common::bevy::ecs::archetype::ArchetypeId;
use ouroboros_common::bevy::ecs::component::{ComponentId, ComponentTicks, StorageType};
use ouroboros_common::bevy::pbr::CubemapVisibleEntities;
use ouroboros_common::bevy::prelude::{
    Component, Deref, DerefMut, Entity, ReflectComponent, World,
};
use ouroboros_common::bevy::reflect::TypeRegistry;
use ouroboros_common::bevy::render::camera::Camera3d;
use ouroboros_common::bevy::render::primitives::{CubemapFrusta, Frustum};
use ouroboros_common::bevy::render::view::VisibleEntities;
use ouroboros_common::bevy::utils::{HashMap, HashSet};
use ouroboros_common::message::SceneDiff;
use ouroboros_common::quinn::Endpoint;
use ouroboros_common::{Interface, Message, ReflectObject, StreamCounter, StreamId};

use super::client_config;

pub async fn run_client(
    local_rx: Receiver<(StreamId, Box<dyn Message>)>,
    remote_tx: Sender<(StreamId, Box<dyn Message>)>,
    mut stream_counter: StreamCounter,
) -> Result<(), RemoteThreadError> {
    let endpoint = Endpoint::client(ouroboros_common::client_addr())?;

    println!("Attempting connection!");

    let new = endpoint
        .connect_with(
            client_config(),
            ouroboros_common::server_addr(),
            "localhost",
        )?
        .await?;

    println!("Acquired connection to editor!");

    asynchronous::process_connection(new, &local_rx, &remote_tx, &mut stream_counter).await?;

    Ok(())
}

#[derive(Default)]
struct SceneDiffState {
    map: HashMap<Entity, Vec<ReflectObject>>,
    last_change_tick: u32,
}

#[derive(Debug, Default, Deref, DerefMut)]
pub struct SceneDiffDenylist(HashSet<ComponentId>);

impl SceneDiffDenylist {
    pub fn deny<T: Component>(&mut self, world: &World) -> Option<()> {
        self.insert(world.components().get_id(TypeId::of::<T>())?);
        Some(())
    }
}

// TODO: A custom serializer might be a more performant alternative to this
pub fn send_scene_diff(world: &mut World) {
    let mut state = world
        .remove_resource::<SceneDiffState>()
        .unwrap_or_default();
    let denylist = world
        .remove_resource::<SceneDiffDenylist>()
        .unwrap_or_default();

    let change_tick = world.change_tick();
    let registry = world.remove_resource::<TypeRegistry>().unwrap();
    let interface = world.remove_resource::<Interface>().unwrap();
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
            if denylist.contains(&component_id) {
                continue;
            }

            // Safe: The component ID is valid
            let type_id = match unsafe { components.get_info_unchecked(component_id) }.type_id() {
                Some(id) => id,
                None => continue,
            };
            let registration = match reg.get(type_id) {
                Some(reg) => reg,
                None => continue,
            };
            let reflect_component = match registration.data::<ReflectComponent>() {
                Some(data) => data,
                None => continue,
            };

            let storage_type = archetype.get_storage_type(component_id).unwrap();

            match storage_type {
                StorageType::Table => {
                    let column = table.get_column(component_id).unwrap();

                    for entity in archetype.entities() {
                        let location = entities.get(*entity).unwrap();
                        let table_row = archetype.entity_table_row(location.index);
                        // Safe: the table row is obtained safely from the world's state
                        let ticks = unsafe { column.get_ticks_unchecked(table_row) };

                        collect_if_changed(
                            ticks,
                            change_tick,
                            state.last_change_tick,
                            &mut state.map,
                            world,
                            entity,
                            reflect_component,
                        );
                    }
                }
                StorageType::SparseSet => {
                    let sparse_set = storages.sparse_sets.get(component_id).unwrap();

                    for entity in archetype.entities() {
                        let ticks = sparse_set.get_ticks(*entity).unwrap();

                        collect_if_changed(
                            ticks,
                            change_tick,
                            state.last_change_tick,
                            &mut state.map,
                            world,
                            entity,
                            reflect_component,
                        );
                    }
                }
            }
        }
    }

    let msg = SceneDiff {
        changes: state.map.drain().map(|(e, v)| (e.into(), v)).collect(),
    };

    let result = interface.send(None, Box::new(msg));
    if let Ok(id) = result {
        if let Err(err) = interface.close(id) {
            eprintln!("Failed to close scene diff stream with error {:?}", err);
        }
    } else if let Err(err) = result {
        eprintln!("Failed to send scene diff with error {:?}", err);
    }

    state.last_change_tick = change_tick;

    world.insert_resource(state);
    world.insert_resource(denylist);
    drop(reg);
    world.insert_resource(registry);
    world.insert_resource(interface);

    fn collect_if_changed(
        ticks: &ComponentTicks,
        change_tick: u32,
        last_change_tick: u32,
        map: &mut HashMap<Entity, Vec<ReflectObject>>,
        world: &World,
        entity: &Entity,
        reflect_component: &ReflectComponent,
    ) -> Option<()> {
        // TODO: Need someone who knows more about bevy_ecs to review this
        if ticks.is_changed(last_change_tick, change_tick) {
            let reflect = reflect_component
                .reflect_component(world, *entity)?
                .clone_value()
                .into();

            match map.get_mut(entity) {
                Some(vec) => vec.push(reflect),
                None => _ = map.insert(*entity, vec![reflect]),
            }
        }

        Some(())
    }
}

pub fn build_denylist(world: &mut World) {
    let mut denylist = SceneDiffDenylist::default();

    // NOTE: ZSTs should not be denied. They should only be changed 
    // when added or removed, and so should not cause any unnecessary noise.
    // Types which only contain ignored fields, however, should be denied, 
    // as they may change frequently without yielding usable data.
    denylist.deny::<CubemapVisibleEntities>(world).unwrap();
    denylist.deny::<VisibleEntities>(world).unwrap();
    denylist.deny::<Frustum>(world).unwrap();
    denylist.deny::<CubemapFrusta>(world).unwrap();
    world.insert_resource(denylist);
}
