# Bevy Image Export

[![Crates.io](https://img.shields.io/crates/v/bevy_image_export.svg)](https://crates.io/crates/bevy_image_export)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/paulkre/bevy_image_export/blob/main/LICENSE)

A Bevy plugin enabling you to render image sequences.

## Usage

```rust
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
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })

        // Add a child camera to your main camera and insert the ImageExportCamera component.
        .with_children(|parent| {
            parent
                .spawn(Camera3dBundle::default())
                .insert(ImageExportCamera {
                    // The rendered frames will be saved to "./out/[#####].png".
                    output_dir: "out",
                    extension: "png",
                });
        });

    // ...
}
```

## Creating a MP4 file

With [FFmpeg](https://ffmpeg.org) installed, you can run the following command to convert your exported image sequence to a MP4 video file:

```bash
ffmpeg -r 60 -i out/%05d.png -vcodec libx264 -crf 25 -pix_fmt yuv420p out.mp4
```
