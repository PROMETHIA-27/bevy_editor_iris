use bevy_egui::egui;
use ouroboros_common::bevy::prelude::{FromWorld, World};
use ouroboros_common::RemoteEntity;

use crate::{server::EntityCache, tabs::EditorTab};

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
        egui::SidePanel::left("Entity List").show(ui.ctx(), |ui| {
            let cache = self.entities.read().unwrap();
            for (entity, name) in cache.iter() {
                let selected = self.selected_entity == Some(*entity);
                if ui
                    .selectable_label(selected, name.as_ref().unwrap_or(&entity.to_string()))
                    .clicked()
                {
                    self.selected_entity.replace(*entity);
                }
            }
        });
    }
}
