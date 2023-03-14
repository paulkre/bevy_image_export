use super::node::{ImageExportNode, NODE_NAME};
use bevy::{
    ecs::query::QueryItem,
    prelude::*,
    render::{
        camera::{CameraUpdateSystem, RenderTarget, Viewport},
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        main_graph::node::CAMERA_DRIVER,
        render_graph::RenderGraph,
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, Extent3d, MapMode, TextureDescriptor,
            TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
};
use futures::channel::oneshot;
use image::{ImageBuffer, Rgba};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wgpu::Maintain;

/// Any camera entity holding this component will render its view to an image sequence in the file system.
#[derive(Component, Clone)]
pub struct ImageExportCamera {
    /// The directory that image files will be saved to.
    pub output_dir: &'static str,
    /// The image file extension. Supported extensions are listed [here](https://github.com/image-rs/image#supported-image-formats).
    pub extension: &'static str,
    /// The resolution of the output image. If none, viewport resolution is used.
    pub resolution: Option<UVec2>,
}

impl Default for ImageExportCamera {
    fn default() -> Self {
        Self {
            output_dir: "out",
            extension: "png",
            resolution: None,
        }
    }
}

#[derive(Component, Clone)]
pub struct ImageExportTask {
    pub render_target: Handle<Image>,
    pub output_buffer: Buffer,
    pub size: UVec2,
}

#[derive(Component, Clone, Default, Deref, DerefMut)]
pub struct ImageExportFrameId(u32);

impl ImageExportTask {
    pub fn new(device: &RenderDevice, render_target: Handle<Image>, size: UVec2) -> Self {
        let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row((size.x) as usize) * 4;

        Self {
            render_target,
            size,
            output_buffer: device.create_buffer(&BufferDescriptor {
                label: Some("output_buffer"),
                size: padded_bytes_per_row as u64 * size.y as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
        }
    }
}

pub fn setup_export_task(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(Entity, &ImageExportCamera, &mut Camera), Without<ImageExportTask>>,
    device: Res<RenderDevice>,
) {
    for (entity, settings, mut camera) in query.iter_mut() {
        let resolution = settings.resolution.unwrap_or_else(|| {
            camera
                .physical_target_size()
                .expect("Could not determine export resolution")
        });

        let extent = Extent3d {
            width: resolution.x,
            height: resolution.y,
            ..default()
        };

        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size: extent,
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
        image.resize(extent);

        let image_handle = images.add(image);

        camera.target = RenderTarget::Image(image_handle.clone());
        camera.viewport = Some(Viewport {
            physical_size: resolution,
            ..default()
        });

        commands
            .entity(entity)
            .insert(ImageExportTask::new(&device, image_handle, resolution));
    }
}

impl ExtractComponent for ImageExportCamera {
    type Query = (
        &'static Self,
        &'static ImageExportTask,
        &'static ImageExportFrameId,
    );
    type Filter = ();
    type Out = (Self, ImageExportTask, ImageExportFrameId);

    fn extract_component((cam, task, id): QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        Some((cam.clone(), task.clone(), id.clone()))
    }
}

#[derive(Default, Clone, Resource)]
pub struct ExportThreads {
    pub count: Arc<AtomicUsize>,
}

impl ExportThreads {
    /// Blocks the main thread until all frames have been saved successfully.
    pub fn finish(&self) {
        while self.count.load(Ordering::SeqCst) > 0 {
            std::thread::sleep(std::time::Duration::from_secs_f32(0.25));
        }
    }
}

pub fn update_frame_id(
    mut commands: Commands,
    frame_ids: Query<(Entity, Option<&ImageExportFrameId>), With<ImageExportCamera>>,
) {
    for (entity, frame_id) in frame_ids.iter() {
        let mut frame_id = frame_id.cloned().unwrap_or_default();
        (*frame_id) = frame_id.wrapping_add(1);
        commands.entity(entity).insert(frame_id);
    }
}

pub fn export_image(
    tasks: Query<(&ImageExportTask, &ImageExportFrameId, &ImageExportCamera)>,
    render_device: Res<RenderDevice>,
    export_threads: Res<ExportThreads>,
) {
    for (task, frame_id, settings) in tasks.iter() {
        let data = {
            let slice = task.output_buffer.slice(..);

            {
                let (mapping_tx, mapping_rx) = oneshot::channel();

                render_device.map_buffer(&slice, MapMode::Read, move |res| {
                    mapping_tx.send(res).unwrap();
                });

                render_device.poll(Maintain::Wait);
                futures_lite::future::block_on(mapping_rx).unwrap().unwrap();
            }

            slice.get_mapped_range().to_vec()
        };

        task.output_buffer.unmap();

        let frame_id = **frame_id;
        let export_threads = export_threads.clone();
        let size = task.size;
        let settings = settings.clone();

        export_threads.count.fetch_add(1, Ordering::SeqCst);
        std::thread::spawn(move || {
            save_image_file(data, size, frame_id, settings);
            export_threads.count.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

fn save_image_file(data: Vec<u8>, size: UVec2, frame_id: u32, settings: ImageExportCamera) {
    std::fs::create_dir_all(settings.output_dir).expect("Output path could not be created");

    let path = format!(
        "{}/{:05}.{}",
        settings.output_dir, frame_id, settings.extension
    );

    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(size.x, size.y, data).unwrap();
    buffer.save(path).unwrap();
}

/// Plugin enabling the generation of image sequences.
#[derive(Default)]
pub struct ImageExportPlugin {
    pub threads: ExportThreads,
}

impl Plugin for ImageExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ExtractComponentPlugin::<ImageExportCamera>::default())
            .add_systems(
                (
                    setup_export_task.before(CameraUpdateSystem),
                    update_frame_id,
                )
                    .in_base_set(CoreSet::PostUpdate),
            );

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .insert_resource(self.threads.clone())
            .add_system(export_image.in_set(RenderSet::RenderFlush));

        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        graph.add_node(NODE_NAME, ImageExportNode::default());
        graph.add_node_edge(CAMERA_DRIVER, NODE_NAME);
    }
}
