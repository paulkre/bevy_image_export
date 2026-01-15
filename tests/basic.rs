use anyhow::anyhow;
use bevy::{
    app::plugin_group,
    camera::RenderTarget,
    prelude::*,
    render::{
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        RenderPlugin,
    },
};
use bevy_image_export::{ImageExport, ImageExportPlugin, ImageExportSource};
use std::f32::consts::PI;

const WIDTH: u32 = 16;
const HEIGHT: u32 = 16;

/// Number of frames to wait before starting the export. This is necessary because Bevy doesn't
/// predictably start rendering on the first frame.
const STARTUP_FRAMES: u32 = 2;

plugin_group! {
    pub struct TestPlugins {
        bevy::app:::PanicHandlerPlugin,
        bevy::log:::LogPlugin,
        bevy::app:::TaskPoolPlugin,
        bevy::diagnostic:::FrameCountPlugin,
        bevy::time:::TimePlugin,
        bevy::transform:::TransformPlugin,
        bevy::app:::ScheduleRunnerPlugin,
        bevy::window:::WindowPlugin,
        bevy::asset:::AssetPlugin,
        bevy::render:::RenderPlugin,
        bevy::image:::ImagePlugin,
        bevy::mesh:::MeshPlugin,
        bevy::camera:::CameraPlugin,
        bevy::light:::LightPlugin,
        bevy::render::pipelined_rendering:::PipelinedRenderingPlugin,
        bevy::core_pipeline:::CorePipelinePlugin,
        bevy::post_process:::PostProcessPlugin,
        bevy::pbr:::PbrPlugin,
    }
}

fn open_image(path: &str) -> anyhow::Result<Vec<u8>> {
    Ok(image::open(path)
        .map_err(|e| anyhow::anyhow!("failed to open {}: {}", path, e))?
        .into_rgb8()
        .into_raw())
}

fn assert_image_eq(a: &[u8], b: &[u8]) -> anyhow::Result<()> {
    if a.len() != b.len() {
        anyhow::bail!("images are not equal");
    }

    let mut error: usize = 0;
    for (a, b) in a.iter().zip(b.iter()) {
        error += (*a as i32 - *b as i32).unsigned_abs() as usize;
    }

    if error > 20 {
        anyhow::bail!("images are not equal, error: {}", error);
    }

    Ok(())
}

#[derive(Resource, Debug)]
struct ImageCount(u32);

#[test]
fn test_basic() -> anyhow::Result<()> {
    let export_plugin = ImageExportPlugin::default();
    let export_threads = export_plugin.threads.clone();
    let image_count = 5;

    App::new()
        .add_plugins((
            TestPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (WIDTH, HEIGHT).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..Default::default()
                }),
            export_plugin,
        ))
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 1000.0,
            affects_lightmapped_meshes: true,
        })
        .insert_resource(ImageCount(image_count))
        .add_systems(Update, (setup, update).chain())
        .run();

    export_threads.finish();

    for i in 1..=image_count {
        let filename = format!("{:05}.png", i);
        assert_image_eq(
            &open_image(&format!("./out/{}", filename))?,
            &open_image(&format!("./tests/fixtures/basic/{}", filename))?,
        )
        .map_err(|e| anyhow!("{}: {}", filename, e))?;
    }

    Ok(())
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut export_sources: ResMut<Assets<ImageExportSource>>,
    mut frame: Local<u32>,
) {
    *frame += 1;
    let frame = *frame as i32 - STARTUP_FRAMES as i32;
    if frame != 1 {
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
        Transform::from_translation(4.2 * Vec3::Z),
        children![(
            Camera3d::default(),
            RenderTarget::Image(output_texture_handle.clone().into()),
            Camera {
                clear_color: ClearColorConfig::Custom(Color::BLACK),
                ..default()
            },
        )],
    ));

    commands.spawn(ImageExport(export_sources.add(output_texture_handle)));

    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Cuboid::default()))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.0, 0.0))),
        Moving,
    ));
}

#[derive(Component)]
struct Moving;
fn update(
    image_count: Res<ImageCount>,
    mut app_exit_events: MessageWriter<AppExit>,
    mut frame: Local<u32>,
    mut transforms: Query<&mut Transform, With<Moving>>,
) {
    *frame += 1;
    let frame = *frame as i32 - STARTUP_FRAMES as i32;
    if frame < 1 {
        return;
    }

    let theta = (frame - 1) as f32 * 0.25 * PI;
    for mut transform in &mut transforms {
        transform.translation = Vec3::new(theta.sin(), theta.cos(), 0.0);
    }

    if frame >= (image_count.0 as i32) {
        app_exit_events.write(AppExit::Success);
    }
}
