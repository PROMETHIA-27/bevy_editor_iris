use ::bevy_mod_ouroboros::*;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(OuroborosClientPlugin)
        .add_startup_system(setup)
        .run()
}

fn setup(mut c: Commands, mut m: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let mesh = m.add(shape::Cube { size: 1.0 }.into());
    let material = mats.add(Color::WHITE.into());

    c.spawn_bundle(PbrBundle {
        mesh,
        material,
        ..default()
    });

    c.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    c.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(1.0, 2.0, 3.0),
        ..default()
    });
}
