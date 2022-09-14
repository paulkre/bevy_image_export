use bevy::{
    prelude::*,
    render::{camera::RenderTarget, render_resource::*, renderer::RenderDevice, Extract},
};
use futures::channel::oneshot;
use image::{ImageBuffer, Rgba};

use super::plugin::ExportThreads;

/// Any camera entity holding this component will render its view to an image sequence in the file system.
#[derive(Component, Clone)]
pub struct ImageExportCamera {
    /// The directory that image files will be saved to.
    pub output_dir: &'static str,
    /// The image file extension. Supported extensions are listed [here](https://github.com/image-rs/image#supported-image-formats).
    pub extension: &'static str,
}

impl Default for ImageExportCamera {
    fn default() -> Self {
        Self {
            output_dir: "out",
            extension: "png",
        }
    }
}

#[derive(Component, Clone)]
pub struct ImageExportTask {
    pub render_target: Handle<Image>,
    pub output_buffer: Buffer,
    pub size: UVec2,
    pub frame_id: u32,
}

impl ImageExportTask {
    pub fn new(device: &RenderDevice, render_target: Handle<Image>, size: UVec2) -> Self {
        Self {
            render_target,
            size,
            output_buffer: device.create_buffer(&BufferDescriptor {
                label: Some("output_buffer"),
                size: ((std::mem::size_of::<u32>() as u32) * size.x * size.y) as BufferAddress,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            frame_id: 1,
        }
    }
}

pub fn setup_export_data(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut query: Query<(Entity, &mut Camera), (With<ImageExportCamera>, Without<ImageExportTask>)>,
    device: Res<RenderDevice>,
) {
    query.iter_mut().for_each(|(entity, mut camera)| {
        let size = camera
            .physical_target_size()
            .expect("Could not determine export resolution");

        let extent = Extent3d {
            width: size.x,
            height: size.y,
            ..default()
        };

        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size: extent,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::COPY_DST
                    | TextureUsages::COPY_SRC
                    | TextureUsages::RENDER_ATTACHMENT,
            },
            ..default()
        };
        image.resize(extent);

        let image_handle = images.add(image);

        camera.target = RenderTarget::Image(image_handle.clone());

        commands
            .entity(entity)
            .insert(ImageExportTask::new(&device, image_handle, size));
    });
}

pub fn extract_image_export_tasks(
    mut commands: Commands,
    tasks: Extract<Query<(Entity, &ImageExportTask, &ImageExportCamera)>>,
) {
    tasks.iter().for_each(|(entity, data, settings)| {
        commands
            .get_or_spawn(entity)
            .insert_bundle((data.clone(), settings.clone()));
    });
}

pub fn update_frame_id(mut tasks: Query<&mut ImageExportTask>) {
    tasks.iter_mut().for_each(|mut task| {
        task.frame_id = task.frame_id.wrapping_add(1);
    });
}

pub fn export_image(
    tasks: Query<(&ImageExportTask, &ImageExportCamera)>,
    render_device: Res<RenderDevice>,
    export_threads: Res<ExportThreads>,
) {
    tasks.iter().for_each(|(task, settings)| {
        let data = {
            let slice = task.output_buffer.slice(..);

            {
                let (mapping_tx, mapping_rx) = oneshot::channel();

                render_device.map_buffer(&slice, MapMode::Read, move |res| {
                    mapping_tx.send(res).unwrap();
                });

                render_device.poll(wgpu::Maintain::Wait);
                futures_lite::future::block_on(mapping_rx).unwrap().unwrap();
            }

            slice.get_mapped_range().to_vec()
        };

        task.output_buffer.unmap();

        {
            let frame_id = task.frame_id;
            let export_threads = export_threads.clone();
            let size = task.size;
            let settings = settings.clone();

            *export_threads.count.lock().unwrap() += 1;
            std::thread::spawn(move || {
                save_image_file(data, size, frame_id, settings);
                *export_threads.count.lock().unwrap() -= 1;
            });
        }
    });
}

fn save_image_file(mut data: Vec<u8>, size: UVec2, frame_id: u32, settings: ImageExportCamera) {
    bgra_to_rgba(&mut data);
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(size.x, size.y, data).unwrap();

    match std::fs::create_dir_all(settings.output_dir) {
        Err(_) => panic!("Output path could not be created"),
        _ => {}
    }

    buffer
        .save(format!(
            "{}/{:05}.{}",
            settings.output_dir, frame_id, settings.extension
        ))
        .unwrap();
}

fn bgra_to_rgba(data: &mut Vec<u8>) {
    for src in data.chunks_exact_mut(4) {
        let b = src[0];
        src[0] = src[2];
        src[2] = b;
    }
}
