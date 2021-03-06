use std::fs;
use std::time::Duration;

use common::deps::bevy::ecs as bevy_ecs;
use common::deps::bevy::prelude::{
    App, CoreStage, ExclusiveSystemDescriptorCoercion, IntoExclusiveSystem, Plugin, StartupStage,
    SystemLabel, SystemSet,
};
use common::deps::quinn::ClientConfig;
use common::deps::rustls::{Certificate, RootCertStore};
use common::systems as common_systems;
use common::CommonPlugin;

// pub use self::interface::ClientInterfaceExt;
pub use self::systems::SceneDiffDenylist;

mod interface;
mod systems;

pub struct ClientPlugin;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(CommonPlugin(systems::run_client))
            // .add_system_set_to_stage(
            //     CoreStage::PostUpdate,
            //     SystemSet::new()
            //         .with_run_criteria(common_systems::run_on_timer(Duration::from_secs(1)))
            //         .with_system(systems::send_scene_diff.exclusive_system()),
            // )
            .add_startup_system_to_stage(
                StartupStage::PostStartup,
                systems::build_denylist
                    .exclusive_system()
                    .label(BuildDenylist),
            );
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, SystemLabel)]
pub struct BuildDenylist;

fn client_config() -> ClientConfig {
    let cert = Certificate(fs::read("certificate.der").unwrap());

    let mut store = RootCertStore::empty();
    store.add(&cert).unwrap();

    ClientConfig::with_root_certificates(store)
}
