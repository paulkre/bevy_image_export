# Bevy Image Export plugin

A Bevy plugin for exporting your app as an image sequence.

## Usage

```rust
use bevy::{prelude::*, winit::WinitSettings};
use bevy_image_export::{ImageExportCamera, ImageExportPlugin};

fn main() {
    let export_plugin = ImageExportPlugin::default();
    let export_threads = export_plugin.threads.clone();

    App::new()
        .insert_resource(WindowDescriptor {
            width: 1024.,
            height: 1024.,
            ..default()
        })
        .insert_resource(WinitSettings {
            return_from_run: true,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(export_plugin)
        // ...
        .run();

    // This is optional but recommended.
    // It blocks the main thread until all images have been exported.
    export_threads.finish();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        // Add a child camera to your main camera and insert the ImageExportCamera component.
        .with_children(|parent| {
            parent
                .spawn_bundle(Camera3dBundle::default())
                .insert(ImageExportCamera);
        });

    // ...
}
```
