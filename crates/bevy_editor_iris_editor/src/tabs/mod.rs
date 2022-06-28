use std::any::{self, Any, TypeId};

use bevy_egui::egui;
use common::deps::bevy::prelude::{default, App, FromWorld, Plugin};
use common::deps::bevy::reflect::{self as bevy_reflect, Reflect};
use common::deps::bevy::utils::HashMap;

mod inspector;
mod resources;

pub use inspector::InspectorTab;
pub use resources::SelectedTab;

pub struct TabPlugin;

impl Plugin for TabPlugin {
    fn build(&self, app: &mut App) {
        let mut registry = TabRegistry::new();

        let mut inspector = InspectorTab::from_world(&mut app.world);
        inspector.on_register(app);
        registry.push(inspector);

        app.insert_resource(registry)
            .insert_resource(SelectedTab(TypeId::of::<InspectorTab>()));
    }
}

#[derive(Reflect)]
pub struct TabRegistration {
    #[reflect(ignore)]
    tab: Box<dyn AnyTab + Send + Sync>,
    #[reflect(ignore)]
    ty_id: TypeId,
    ty_name: String,
}

impl TabRegistration {
    pub fn name(&self) -> egui::RichText {
        self.tab.name()
    }

    pub fn display(&mut self, ui: &mut egui::Ui) {
        self.tab.display(ui);
    }

    pub fn type_id(&self) -> TypeId {
        self.ty_id
    }
}

// TODO: In a future version of bevy it will be possible to `Reflect` this. 6/9/2022
#[derive(Default)]
pub struct TabRegistry {
    pub registrations: HashMap<TypeId, (usize, TabRegistration)>,
    pub order: Vec<TypeId>,
}

impl TabRegistry {
    pub fn new() -> Self {
        default()
    }

    pub fn insert<T: EditorTab>(&mut self, index: usize, tab: T) -> &mut Self {
        self.registrations
            .insert(TypeId::of::<T>(), (index, tab.get_registration()));
        self.order.insert(index, TypeId::of::<T>());
        self
    }

    pub fn push<T: EditorTab>(&mut self, tab: T) -> &mut Self {
        self.insert::<T>(self.order.len(), tab);
        self
    }

    pub fn remove<T: EditorTab>(&mut self) -> Option<TabRegistration> {
        let (ord, reg) = self.registrations.remove(&TypeId::of::<T>())?;
        self.order.remove(ord);
        Some(reg)
    }

    pub fn get<T: EditorTab>(&self) -> Option<&T> {
        let (_, reg) = self.registrations.get(&TypeId::of::<T>())?;
        reg.tab.as_any().downcast_ref()
    }

    pub fn get_mut<T: EditorTab>(&mut self) -> Option<&mut T> {
        let (_, reg) = self.registrations.get_mut(&TypeId::of::<T>())?;
        reg.tab.as_mut_any().downcast_mut()
    }

    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

pub trait EditorTab: Any
where
    Self: Any + Send + Sync,
{
    fn name(&self) -> egui::RichText;

    fn display(&mut self, ui: &mut egui::Ui);

    fn on_register(&mut self, _app: &mut App) {}
}

trait AnyTab: EditorTab {
    fn as_any(&self) -> &dyn Any;

    fn as_mut_any(&mut self) -> &mut dyn Any;
}

impl<T: EditorTab> AnyTab for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
}

trait IntoTabRegistration
where
    Self: Sized + EditorTab,
{
    fn get_registration(self) -> TabRegistration;
}

impl<T: EditorTab> IntoTabRegistration for T {
    fn get_registration(self) -> TabRegistration {
        TabRegistration {
            tab: Box::new(self),
            ty_id: TypeId::of::<Self>(),
            ty_name: any::type_name::<Self>().into(),
        }
    }
}
