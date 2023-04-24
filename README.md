# Bevy Image Export

[![Crates.io](https://img.shields.io/crates/v/bevy_image_export.svg)](https://crates.io/crates/bevy_image_export)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/paulkre/bevy_image_export/blob/main/LICENSE)

A Bevy plugin for rendering image sequences.

## Compatability

| Bevy Version | Crate Version       |
| ------------ | ------------------- |
| `0.10`       | `0.4`, `0.5`, `0.6` |
| `0.9`        | `0.3`               |
| `0.8`        | `0.1`, `0.2`        |

## Usage

```rust
use bevy::{prelude::*, winit::WinitSettings};
use bevy_image_export::{ImageExportPlugin, ImageExporterBundle, ImageExporterSource};

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
                resolution: (768.0, 768.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugin(export_plugin)
        // ...
        .run();

    // This line is optional but recommended.
    // It blocks the main thread until all image files have been saved successfully.
    export_threads.finish();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut exporter_sources: ResMut<Assets<ImageExporterSource>>,
) {
    // Create an output texture.
    let output_texture_handle = {
        let size = Extent3d {
            width: 768,
            height: 768,
            ..default()
        };
        let mut export_texture = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC
                    | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        export_texture.resize(size);

        images.add(export_texture)
    };

    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_translation(5.0 * Vec3::Z),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(Camera3dBundle {
                camera: Camera {
                    // Connect the output texture to a camera as a RenderTarget.
                    target: RenderTarget::Image(output_texture_handle.clone()),
                    ..default()
                },
                ..default()
            });
        });

    // Spawn the ImageExporterBundle to initiate the export of the output texture.
    commands.spawn(ImageExporterBundle {
        source: exporter_sources.add(output_texture_handle.into()),
        settings: ImageExporterSettings {
            // Frames will be saved to "./out/[#####].png".
            output_dir: "out".into(),
            extension: "png".into(),
        },
    });

    // ...
}
```

## Video file export

With [FFmpeg](https://ffmpeg.org) installed, you can run the following command to convert your exported image sequence to an MP4 video file:

```bash
ffmpeg -r 60 -i out/%05d.png -vcodec libx264 -crf 25 -pix_fmt yuv420p out.mp4
```
