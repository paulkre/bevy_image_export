use bevy::{
    prelude::*,
    render::{camera::RenderTarget, render_resource::*, renderer::RenderDevice, Extract},
};
use futures::channel::oneshot;
use image::{ImageBuffer, Rgba};

use super::plugin::ExportThreads;

#[derive(Component, Default)]
pub struct ImageExportCamera;

#[derive(Component, Clone)]
pub struct ImageExportTask {
    pub render_target: Handle<Image>,
    pub output_buffer: Buffer,
    pub size: UVec2,
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
    })
}

pub fn extract_image_export_tasks(
    mut commands: Commands,
    tasks: Extract<Query<(Entity, &ImageExportTask)>>,
) {
    tasks.iter().for_each(|(entity, data)| {
        commands.get_or_spawn(entity).insert(data.clone());
    });
}

pub fn export_image(
    tasks: Query<&ImageExportTask>,
    render_device: Res<RenderDevice>,
    mut frame_id: Local<u32>,
    export_threads: Res<ExportThreads>,
) {
    *frame_id = frame_id.wrapping_add(1);

    tasks.iter().for_each(
        |ImageExportTask {
             output_buffer,
             size,
             ..
         }| {
            let data = {
                let slice = output_buffer.slice(..);

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

            output_buffer.unmap();

            {
                let frame_id = *frame_id;
                let export_threads = export_threads.clone();
                let size = *size;

                *export_threads.count.lock().unwrap() += 1;
                std::thread::spawn(move || {
                    save_image_file(data, size, "out", frame_id);
                    *export_threads.count.lock().unwrap() -= 1;
                });
            }
        },
    );
}

fn save_image_file(mut data: Vec<u8>, size: UVec2, export_dir: &'static str, frame_id: u32) {
    bgra_to_rgba(&mut data);
    let buffer = ImageBuffer::<Rgba<u8>, _>::from_raw(size.x, size.y, data).unwrap();

    match std::fs::create_dir_all(export_dir) {
        Err(_) => panic!("Output path could not be created"),
        _ => {}
    }

    buffer
        .save(format!("{}/{:05}.png", export_dir, frame_id))
        .unwrap();
}

fn bgra_to_rgba(data: &mut Vec<u8>) {
    for src in data.chunks_exact_mut(4) {
        let r = src[2];
        let g = src[1];
        let b = src[0];
        let a = src[3];
        src[0] = r;
        src[1] = g;
        src[2] = b;
        src[3] = a;
    }
}
