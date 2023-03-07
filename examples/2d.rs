use std::f32::consts::PI;

use bevy::{prelude::*, sprite::MaterialMesh2dBundle, winit::WinitSettings};
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
            primary_window: Some(Window {
                resolution: (1024., 1024.).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugin(export_plugin)
        .add_startup_system(setup)
        .add_system(update)
        .run();

    export_threads.finish();
}

#[derive(Component)]
struct Shape;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
            transform: Transform::default().with_scale(Vec3::splat(64.)),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            ..default()
        })
        .insert(Shape);
}

fn update(
    mut commands: Commands,
    mut frame_id: Local<u32>,
    mut query: Query<&mut Transform, With<Shape>>,
) {
    *frame_id = frame_id.wrapping_add(1);
    let time = (*frame_id as f32) * (1.0 / 60.0);

    if *frame_id == 5 {
        commands
            .spawn(Camera2dBundle::default())
            .insert(ImageExportCamera::default());
    }

    for mut transform in query.iter_mut() {
        transform.translation = Vec3::Y * 384. * (time * PI).sin();
    }
}
