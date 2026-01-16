mod common;

use crate::common::graceperiod::{GracePeriodPlugin, GracefulFrameCount};
use bevy::{
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::Hdr,
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
                        resolution: (WIDTH, HEIGHT).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
            GracePeriodPlugin::default(),
            export_plugin,
        ))
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (setup_camera, update).chain())
        .run();

    export_threads.finish();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(GlobalAmbientLight {
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

fn setup_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut exporter_sources: ResMut<Assets<ImageExportSource>>,
    frame_count: Res<GracefulFrameCount>,
) {
    if frame_count.frame() != 1 {
        return;
    }

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
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        Transform::from_translation(4.2 * Vec3::Z),
        tonemapping,
        children![(
            Camera3d::default(),
            Hdr,
            RenderTarget::Image(output_texture_handle.clone().into()),
            Camera::default(),
            tonemapping,
        )],
    ));

    commands.spawn((
        ImageExport(exporter_sources.add(output_texture_handle)),
        ImageExportSettings {
            extension: "exr".into(),
            ..default()
        },
    ));
}

#[derive(Component)]
struct Moving;
fn update(
    mut transforms: Query<&mut Transform, With<Moving>>,
    frame_count: Res<GracefulFrameCount>,
) {
    let frame = frame_count.frame().wrapping_sub(1);
    let theta = frame as f32 * 0.25 * PI;
    for mut transform in &mut transforms {
        transform.translation = 10.0 * Vec3::new(theta.sin(), theta.cos(), 0.5);
    }
}
