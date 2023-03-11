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
                resolution: (768., 768.).into(),
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
    // Window camera
    commands.spawn(Camera2dBundle::default());

    // Image export camera
    commands
        .spawn(Camera2dBundle::default())
        .insert(ImageExportCamera::default());

    commands
        .spawn(MaterialMesh2dBundle {
            mesh: meshes.add(Mesh::from(shape::Quad::default())).into(),
            transform: Transform::default().with_scale(Vec3::splat(128.)),
            material: materials.add(ColorMaterial::from(Color::PURPLE)),
            ..default()
        })
        .insert(Shape);
}

fn update(mut frame_id: Local<u32>, mut query: Query<&mut Transform, With<Shape>>) {
    let time = (*frame_id as f32) * (1.0 / 60.0);
    *frame_id = frame_id.wrapping_add(1);

    for mut transform in query.iter_mut() {
        transform.translation = Vec3::Y * 256. * (time * PI).sin();
    }
}
