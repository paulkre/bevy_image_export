use std::f32::consts::TAU;

use bevy::{prelude::*, winit::WinitSettings};
use bevy_image_export::{ImageExportCamera, ImageExportPlugin};

fn main() {
    let export_plugin = ImageExportPlugin::default();
    let export_threads = export_plugin.threads.clone();

    App::new()
        .insert_resource(WinitSettings {
            return_from_run: true,
            ..default()
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: 1024.,
                height: 1024.,
                ..default()
            },
            ..default()
        }))
        .add_plugin(export_plugin)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0,
        })
        .add_startup_system(setup)
        .add_system(spin)
        .run();

    export_threads.finish();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(Camera3dBundle::default())
                .insert(ImageExportCamera::default());
        });

    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere::default())),
            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            ..default()
        })
        .with_children(|p| {
            p.spawn(SpatialBundle::VISIBLE_IDENTITY)
                .with_children(|p| {
                    p.spawn(PbrBundle {
                        transform: Transform::from_xyz(1.5, 0.0, 0.0),
                        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.5 })),
                        material: materials.add(Color::rgb(0.3, 0.9, 0.3).into()),
                        ..default()
                    })
                    .insert(Spinning::default());
                })
                .insert(Spinning { speed: 0.25 });
        });
}

#[derive(Component)]
struct Spinning {
    speed: f32,
}

impl Default for Spinning {
    fn default() -> Self {
        Self { speed: 1.0 }
    }
}

fn spin(mut query: Query<(&mut Transform, &Spinning)>) {
    let angle = TAU * (1.0 / 60.0);
    query.iter_mut().for_each(|(mut transform, spin_settings)| {
        transform.rotate_y(-angle * spin_settings.speed);
    });
}
