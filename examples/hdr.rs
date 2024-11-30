use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        RenderPlugin,
    },
};
use bevy_image_export::{ImageExport, ImageExportPlugin, ImageExportSettings, ImageExportSource};
use std::f32::consts::PI;

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
                        resolution: (WIDTH as f32, HEIGHT as f32).into(),
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
        .add_systems(Update, update)
        .run();

    export_threads.finish();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut exporter_sources: ResMut<Assets<ImageExportSource>>,
) {
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
                format: TextureFormat::Rgba32Float,
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

    let tonemapping = Tonemapping::None;
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                hdr: true,
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
            Transform::from_translation(4.2 * Vec3::Z),
            tonemapping,
        ))
        .with_child((
            Camera3d::default(),
            Camera {
                hdr: true,
                target: RenderTarget::Image(output_texture_handle.clone()),
                ..default()
            },
            tonemapping,
        ));

    commands.spawn((
        ImageExport(exporter_sources.add(output_texture_handle)),
        ImageExportSettings {
            extension: "exr".into(),
            ..default()
        },
    ));

    commands.insert_resource(AmbientLight {
        brightness: 0.0,
        ..default()
    });

    commands.spawn((
        PointLight {
            intensity: 100_000_000.0,
            ..default()
        },
        Moving,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Sphere { radius: 1.0 }))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.75, 0.5))),
    ));
}

#[derive(Component)]
struct Moving;
fn update(mut transforms: Query<&mut Transform, With<Moving>>, mut frame: Local<u32>) {
    let theta = *frame as f32 * 0.25 * PI;
    *frame += 1;
    for mut transform in &mut transforms {
        transform.translation = 10.0 * Vec3::new(theta.sin(), theta.cos(), 0.5);
    }
}
