# Bevy Image Export

[![Crates.io](https://img.shields.io/crates/v/bevy_image_export.svg)](https://crates.io/crates/bevy_image_export)
[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/paulkre/bevy_image_export/blob/main/LICENSE)

A [Bevy](https://bevyengine.org/) plugin for rendering image sequences.

## Compatibility

| Bevy Version | Crate Version  |
| ------------ | -------------- |
| `0.18`       | `0.16`         |
| `0.17`       | `0.15`         |
| `0.16`       | `0.13`, `0.14` |
| `0.15`       | `0.12`         |
| `0.14`       | `0.11`         |
| `0.13`       | `0.10`         |
| `0.12`       | `0.9`          |
| `0.11`       | `0.8`          |
| `0.10`       | `0.4` - `0.7`  |
| `0.9`        | `0.3`          |
| `0.8`        | `0.1`, `0.2`   |

## Usage

```rust
const WIDTH: u32 = 768;
const HEIGHT: u32 = 768;

fn main() {
    let export_plugin = ImageExportPlugin::default();
    let export_threads = export_plugin.threads.clone();

    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (WIDTH, HEIGHT).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
            export_plugin,
        ))
        .add_systems(Startup, setup)
        .run();

    // This line is optional but recommended.
    // It blocks the main thread until all image files have been saved successfully.
    export_threads.finish();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut export_sources: ResMut<Assets<ImageExportSource>>,
) {
    // Create an output texture.
    let output_texture_handle = {
        let size = Extent3d {
            width: WIDTH,
            height: HEIGHT,
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

    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(5.0 * Vec3::Z),
        children![(
            Camera3d::default(),
            // Connect the output texture to a camera as a RenderTarget.
            RenderTarget::Image(output_texture_handle.clone().into()),
            Camera::default(),
        )],
    ));

    // Spawn the ImageExport component to initiate the export of the output texture.
    commands.spawn((
        ImageExport(export_sources.add(output_texture_handle)),
        ImageExportSettings {
            // Frames will be saved to "./out/[#####].png".
            output_dir: "out".into(),
            // Choose "exr" for HDR renders.
            extension: "png".into(),
        },
    ));
}
```

## Video file export

With [FFmpeg](https://ffmpeg.org) installed, you can run the following command to convert your exported image sequence to an MP4 video file:

```bash
ffmpeg -r 60 -i out/%05d.png -vcodec libx264 -crf 25 -pix_fmt yuv420p out.mp4
```
