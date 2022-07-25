use bevy_egui::{egui, EguiContext};
use common::deps::bevy::prelude::World;

use crate::tabs::{SelectedTab, TabRegistry};

pub fn ui(world: &mut World) {
    let ctx = world.remove_resource::<EguiContext>();
    let tabs = world.remove_resource::<TabRegistry>();
    let selected = world.remove_resource::<SelectedTab>();
    let (mut ctx, mut tabs, mut selected) = match (ctx, tabs, selected) {
        (Some(ctx), Some(tabs), Some(selected)) => (ctx, tabs, selected),
        _ => return,
    };

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

        tab.display(ui, world);
    });

    world.insert_resource(ctx);
    world.insert_resource(tabs);
    world.insert_resource(selected);
}
