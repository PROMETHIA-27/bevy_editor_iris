use common::deps::bevy::reflect::Reflect;
use common::prelude::{ReflectObject, RemoteEntity};

pub struct InspectorCache {
    entities: Vec<RemoteEntity>,
    selected: Option<RemoteEntity>,
    selected_components: Vec<ReflectObject>,
}

impl InspectorCache {
    pub fn entities(&self) -> &[RemoteEntity] {
        &self.entities
    }

    pub fn selected(&self) -> &Option<RemoteEntity> {
        &self.selected
    }

    pub fn selected_components(&self) -> &[ReflectObject] {
        &self.selected_components
    }

    pub fn insert_component(&mut self, comp: ReflectObject) {
        let index = self
            .selected_components
            .partition_point(|elem| elem.type_name() < comp.type_name());
        self.selected_components.insert(index, comp);
    }

    pub fn insert_entity(&mut self, entity: RemoteEntity) {
        let index = self.entities.partition_point(|elem| elem < &entity);
        self.entities.insert(index, entity);
    }

    pub fn select(&mut self, entity: RemoteEntity) {
        assert!(
            self.entities.binary_search(&entity).is_ok(),
            "attempted to select an entity not contained in the inspector cache"
        );

        self.selected = Some(entity);
    }

    pub fn deselect(&mut self) {
        self.selected = None;
        self.selected_components.clear();
    }
}
