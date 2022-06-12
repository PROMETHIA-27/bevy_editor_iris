use super::*;
use crate::common::RemoteEntity;

#[derive(Default)]
pub struct InspectorTab {
    selected_entity: Option<RemoteEntity>,
    entity_list: Vec<RemoteEntity>,
    entity_names: Vec<Option<String>>,
}

impl InspectorTab {
    pub fn new() -> Self {
        InspectorTab {
            selected_entity: None,
            entity_list: vec![],
            entity_names: vec![],
        }
    }
}

impl EditorTab for InspectorTab {
    fn name(&self) -> bevy_egui::egui::RichText {
        "Inspector".into()
    }

    fn display(&mut self, ui: &mut egui::Ui) {
        egui::SidePanel::left("Entity List").show(ui.ctx(), |ui| {
            for (index, entity) in self.entity_list.iter().enumerate() {
                if ui
                    .selectable_label(
                        false,
                        self.entity_names[index]
                            .as_ref()
                            .unwrap_or(&entity.to_string()),
                    )
                    .clicked()
                {
                    self.selected_entity.replace(*entity);
                }
            }
        });
    }
}
