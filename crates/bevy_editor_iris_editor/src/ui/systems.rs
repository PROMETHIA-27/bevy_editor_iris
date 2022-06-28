use bevy_egui::{egui, EguiContext};
use common::deps::bevy::prelude::ResMut;

use crate::tabs::{SelectedTab, TabRegistry};

pub fn ui(
    mut ctx: ResMut<EguiContext>,
    mut tabs: ResMut<TabRegistry>,
    mut selected: ResMut<SelectedTab>,
) {
    egui::TopBottomPanel::top("Tabs").show(ctx.ctx_mut(), |ui| {
        ui.horizontal_wrapped(|ui| {
            for tab_id in tabs.order.iter() {
                let (_, tab) = &tabs.registrations[tab_id]; // TODO: Improve tab registry API
                if ui
                    .selectable_label(tab_id == &selected.0, tab.name())
                    .clicked()
                {
                    selected.0 = tab.type_id();
                }
            }
        });
    });

    egui::CentralPanel::default().show(ctx.ctx_mut(), |ui| {
        let (_, tab) = tabs
            .registrations
            .get_mut(&selected)
            .expect("tab removed from registry before being deselected");

        tab.display(ui);
    });
}
