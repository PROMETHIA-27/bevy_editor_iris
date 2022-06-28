use std::any::{self, TypeId};
use std::borrow::Cow;

use bevy_egui::egui::{self, Ui};
use common::deps::bevy::asset::HandleId;
use common::deps::bevy::math::{Mat3, Mat4, Quat, Vec2, Vec3, Vec3A, Vec4};
use common::deps::bevy::prelude::{Color, FromWorld, Name, World};
use common::deps::bevy::reflect::{GetPath, Reflect, ReflectRef};
use common::serde::RemoteEntity;

use crate::server::EntityCache;
use crate::tabs::EditorTab;

pub struct InspectorTab {
    selected_entity: Option<RemoteEntity>,
    entities: EntityCache,
}

impl FromWorld for InspectorTab {
    fn from_world(world: &mut World) -> Self {
        let cache = world.get_resource::<EntityCache>().unwrap();
        Self {
            selected_entity: None,
            entities: cache.clone(),
        }
    }
}

impl EditorTab for InspectorTab {
    fn name(&self) -> bevy_egui::egui::RichText {
        "Inspector".into()
    }

    fn display(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("Entity List")
            .resizable(true)
            .show_inside(ui, |ui| {
                egui::ScrollArea::new([true, true])
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let cache = self.entities.read().unwrap();
                        for (entity, components) in cache.iter() {
                            let selected = self.selected_entity == Some(*entity);
                            let name = components.get(any::type_name::<Name>()).and_then(|name| {
                                name.get_path::<Cow<'static, str>>("name")
                                    .ok()
                                    .map(|s| s.to_string())
                            });
                            if ui
                                .selectable_label(
                                    selected,
                                    name.unwrap_or_else(|| entity.to_string()),
                                )
                                .clicked()
                            {
                                self.selected_entity.replace(*entity);
                            }
                        }
                    });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::new([true, true])
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        if let Some(entity) = &self.selected_entity {
                            let cache = self.entities.read().unwrap();
                            if let Some(components) = cache.get(entity) {
                                for (name, component) in components.iter() {
                                    egui::CollapsingHeader::new(name)
                                        .default_open(true)
                                        .show(ui, |ui| display_component(ui, &*component));
                                }
                            }
                        }
                    });
                });
        });
    }
}

fn display_component(ui: &mut Ui, component: &dyn Reflect) {
    match component.reflect_ref() {
        ReflectRef::Struct(comp) => {
            for i in 0..comp.field_len() {
                egui::CollapsingHeader::new(comp.name_at(i).unwrap())
                    .default_open(true)
                    .show(ui, |ui| display_component(ui, comp.field_at(i).unwrap()));
            }
        }
        ReflectRef::TupleStruct(comp) => {
            for i in 0..comp.field_len() {
                egui::CollapsingHeader::new(i.to_string())
                    .default_open(true)
                    .show(ui, |ui| display_component(ui, comp.field(i).unwrap()));
            }
        }
        ReflectRef::Tuple(comp) => {
            for i in 0..comp.field_len() {
                egui::CollapsingHeader::new(i.to_string())
                    .default_open(true)
                    .show(ui, |ui| display_component(ui, comp.field(i).unwrap()));
            }
        }
        ReflectRef::List(comp) => {
            for i in 0..comp.len() {
                egui::CollapsingHeader::new(i.to_string())
                    .default_open(true)
                    .show(ui, |ui| display_component(ui, comp.get(i).unwrap()));
            }
        }
        ReflectRef::Map(comp) => {
            for i in 0..comp.len() {
                let (key, value) = comp.get_at(i).unwrap();
                egui::CollapsingHeader::new("key")
                    .default_open(true)
                    .show(ui, |ui| display_component(ui, key));
                egui::CollapsingHeader::new("value")
                    .default_open(true)
                    .show(ui, |ui| display_component(ui, value));
            }
        }
        // TODO: ReflectDebug should make this not awful
        ReflectRef::Value(comp) => match comp.type_id() {
            id if id == TypeId::of::<bool>() => {
                ui.label(comp.downcast_ref::<bool>().unwrap().to_string());
            }
            id if id == TypeId::of::<u8>() => {
                ui.label(comp.downcast_ref::<u8>().unwrap().to_string());
            }
            id if id == TypeId::of::<u16>() => {
                ui.label(comp.downcast_ref::<u16>().unwrap().to_string());
            }
            id if id == TypeId::of::<u32>() => {
                ui.label(comp.downcast_ref::<u32>().unwrap().to_string());
            }
            id if id == TypeId::of::<u64>() => {
                ui.label(comp.downcast_ref::<u64>().unwrap().to_string());
            }
            id if id == TypeId::of::<usize>() => {
                ui.label(comp.downcast_ref::<usize>().unwrap().to_string());
            }
            id if id == TypeId::of::<i8>() => {
                ui.label(comp.downcast_ref::<i8>().unwrap().to_string());
            }
            id if id == TypeId::of::<i16>() => {
                ui.label(comp.downcast_ref::<i16>().unwrap().to_string());
            }
            id if id == TypeId::of::<i32>() => {
                ui.label(comp.downcast_ref::<i32>().unwrap().to_string());
            }
            id if id == TypeId::of::<i64>() => {
                ui.label(comp.downcast_ref::<i64>().unwrap().to_string());
            }
            id if id == TypeId::of::<isize>() => {
                ui.label(comp.downcast_ref::<isize>().unwrap().to_string());
            }
            id if id == TypeId::of::<f32>() => {
                ui.label(comp.downcast_ref::<f32>().unwrap().to_string());
            }
            id if id == TypeId::of::<f64>() => {
                ui.label(comp.downcast_ref::<f64>().unwrap().to_string());
            }
            id if id == TypeId::of::<String>() => {
                ui.label(comp.downcast_ref::<String>().unwrap());
            }
            id if id == TypeId::of::<Cow<'static, str>>() => {
                ui.label(match comp.downcast_ref::<Cow<'static, str>>().unwrap() {
                    Cow::Borrowed(str) => *str,
                    Cow::Owned(str) => str,
                });
            }
            id if id == TypeId::of::<HandleId>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<HandleId>().unwrap()));
            }
            id if id == TypeId::of::<Color>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<Color>().unwrap()));
            }
            // TODO: These values will become structs (except for quat, which should be unrecognized)
            id if id == TypeId::of::<Vec2>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<Vec2>().unwrap()));
            }
            id if id == TypeId::of::<Vec3>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<Vec3>().unwrap()));
            }
            id if id == TypeId::of::<Vec3A>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<Vec3A>().unwrap()));
            }
            id if id == TypeId::of::<Vec4>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<Vec4>().unwrap()));
            }
            id if id == TypeId::of::<Quat>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<Quat>().unwrap()));
            }
            id if id == TypeId::of::<Mat3>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<Mat3>().unwrap()));
            }
            id if id == TypeId::of::<Mat4>() => {
                ui.label(format!("{:?}", comp.downcast_ref::<Mat4>().unwrap()));
            }
            _ => _ = ui.label(format!("Unrecognized value of type {}", comp.type_name())),
        },
    }
}
