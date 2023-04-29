use crate::node::{ImageExportNode, NODE_NAME};
use bevy::{
    ecs::{
        query::QueryItem,
        system::{lifetimeless::SRes, SystemParamItem},
    },
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::CameraUpdateSystem,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        main_graph::node::CAMERA_DRIVER,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_graph::RenderGraph,
        render_resource::{Buffer, BufferDescriptor, BufferUsages, Extent3d, MapMode},
        renderer::RenderDevice,
        RenderApp, RenderSet,
    },
};
use bytemuck::AnyBitPattern;
use futures::channel::oneshot;
use image::{
    error::UnsupportedErrorKind, EncodableLayout, ImageBuffer, ImageError, Pixel,
    PixelWithColorType, Rgba,
};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wgpu::Maintain;

#[derive(Clone, TypeUuid, Default)]
#[uuid = "d619b2f8-58cf-42f6-b7da-028c0595f7aa"]
pub struct ImageExportSource(pub Handle<Image>);

impl From<Handle<Image>> for ImageExportSource {
    fn from(value: Handle<Image>) -> Self {
        Self(value)
    }
}

#[derive(Component, Clone)]
pub struct ImageExportSettings {
    /// The directory that image files will be saved to.
    pub output_dir: String,
    /// The image file extension. E.g. "png", "jpeg", or "exr".
    pub extension: String,
}

pub struct GpuImageExportSource {
    pub buffer: Buffer,
    pub source_handle: Handle<Image>,
    pub source_size: Extent3d,
    pub bytes_per_row: u32,
    pub padded_bytes_per_row: u32,
}

impl RenderAsset for ImageExportSource {
    type ExtractedAsset = Self;
    type PreparedAsset = GpuImageExportSource;
    type Param = (SRes<RenderDevice>, SRes<RenderAssets<Image>>);

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        extracted_asset: Self::ExtractedAsset,
        (device, images): &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let gpu_image = images.get(&extracted_asset.0).unwrap();

        let size = gpu_image.texture.size();
        let format = gpu_image.texture_format.describe();
        let bytes_per_row =
            (size.width as u32 / format.block_dimensions.0 as u32) * format.block_size as u32;
        let padded_bytes_per_row =
            RenderDevice::align_copy_bytes_per_row(bytes_per_row as usize) as u32;

        let source_size = gpu_image.texture.size();

        Ok(GpuImageExportSource {
            buffer: device.create_buffer(&BufferDescriptor {
                label: Some("Image Export Buffer"),
                size: (source_size.height * padded_bytes_per_row) as u64,
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            source_handle: extracted_asset.0.clone(),
            source_size,
            bytes_per_row,
            padded_bytes_per_row,
        })
    }
}

#[derive(Component, Clone)]
pub struct ImageExportStartFrame(u64);

impl Default for ImageExportSettings {
    fn default() -> Self {
        Self {
            output_dir: "out".into(),
            extension: "png".into(),
        }
    }
}

impl ExtractComponent for ImageExportSettings {
    type Query = (
        &'static Self,
        &'static Handle<ImageExportSource>,
        &'static ImageExportStartFrame,
    );
    type Filter = ();
    type Out = (Self, Handle<ImageExportSource>, ImageExportStartFrame);

    fn extract_component(
        (settings, source_handle, start_frame): QueryItem<'_, Self::Query>,
    ) -> Option<Self::Out> {
        Some((
            settings.clone(),
            source_handle.clone_weak(),
            start_frame.clone(),
        ))
    }
}

fn setup_exporters(
    mut commands: Commands,
    exporters: Query<Entity, (With<ImageExportSettings>, Without<ImageExportStartFrame>)>,
    mut frame_id: Local<u64>,
) {
    *frame_id = frame_id.wrapping_add(1);
    for entity in &exporters {
        commands
            .entity(entity)
            .insert(ImageExportStartFrame(*frame_id));
    }
}

#[derive(Bundle, Default)]
pub struct ImageExportBundle {
    pub source: Handle<ImageExportSource>,
    pub settings: ImageExportSettings,
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

fn save_buffer_to_disk(
    export_bundles: Query<(
        &Handle<ImageExportSource>,
        &ImageExportSettings,
        &ImageExportStartFrame,
    )>,
    sources: Res<RenderAssets<ImageExportSource>>,
    render_device: Res<RenderDevice>,
    export_threads: Res<ExportThreads>,
    mut frame_id: Local<u64>,
) {
    *frame_id = frame_id.wrapping_add(1);
    for (source_handle, settings, start_frame) in &export_bundles {
        if let Some(gpu_source) = sources.get(source_handle) {
            let mut image_bytes = {
                let slice = gpu_source.buffer.slice(..);

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

            gpu_source.buffer.unmap();

            let settings = settings.clone();
            let frame_id = *frame_id - start_frame.0 + 1;
            let bytes_per_row = gpu_source.bytes_per_row as usize;
            let padded_bytes_per_row = gpu_source.padded_bytes_per_row as usize;
            let source_size = gpu_source.source_size;
            let export_threads = export_threads.clone();

            export_threads.count.fetch_add(1, Ordering::SeqCst);
            std::thread::spawn(move || {
                if bytes_per_row != padded_bytes_per_row {
                    let mut unpadded_bytes =
                        Vec::<u8>::with_capacity(source_size.height as usize * bytes_per_row);

                    for padded_row in image_bytes.chunks(padded_bytes_per_row) {
                        unpadded_bytes.extend_from_slice(&padded_row[..bytes_per_row]);
                    }

                    image_bytes = unpadded_bytes;
                }

                let path = format!(
                    "{}/{:05}.{}",
                    settings.output_dir, frame_id, settings.extension
                );

                std::fs::create_dir_all(&settings.output_dir)
                    .expect("Output path could not be created");

                fn save_buffer<P: Pixel + PixelWithColorType>(
                    image_bytes: &[P::Subpixel],
                    source_size: &Extent3d,
                    path: &str,
                ) where
                    P::Subpixel: AnyBitPattern,
                    [P::Subpixel]: EncodableLayout,
                {
                    match ImageBuffer::<P, _>::from_raw(
                        source_size.width,
                        source_size.height,
                        image_bytes,
                    ) {
                        Some(buffer) => match buffer.save(path) {
                            Err(ImageError::Unsupported(err)) => {
                                if let UnsupportedErrorKind::Format(hint) = err.kind() {
                                    println!("Image format {} is not supported", hint);
                                }
                            }
                            _ => {}
                        },
                        None => {
                            println!("Failed creating image buffer for '{}'", path);
                        }
                    }
                }

                match settings.extension.as_str() {
                    "exr" => {
                        save_buffer::<Rgba<f32>>(
                            bytemuck::cast_slice(&image_bytes),
                            &source_size,
                            path.as_str(),
                        );
                    }
                    _ => {
                        save_buffer::<Rgba<u8>>(&image_bytes, &source_size, path.as_str());
                    }
                }

                export_threads.count.fetch_sub(1, Ordering::SeqCst);
            });
        }
    }
}

/// Plugin enabling the generation of image sequences.
#[derive(Default)]
pub struct ImageExportPlugin {
    pub threads: ExportThreads,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ImageExportSystems {
    SetupImageExport,
    SetupImageExportFlush,
}

impl Plugin for ImageExportPlugin {
    fn build(&self, app: &mut App) {
        use ImageExportSystems::*;

        app.configure_sets(
            (SetupImageExport, SetupImageExportFlush)
                .chain()
                .in_base_set(CoreSet::PostUpdate)
                .before(CameraUpdateSystem),
        )
        .add_asset::<ImageExportSource>()
        .add_plugin(RenderAssetPlugin::<ImageExportSource>::default())
        .add_plugin(ExtractComponentPlugin::<ImageExportSettings>::default())
        .add_systems((
            setup_exporters.in_set(SetupImageExport),
            apply_system_buffers.in_set(SetupImageExportFlush),
        ));

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .insert_resource(self.threads.clone())
            .add_system(save_buffer_to_disk.after(RenderSet::Render));

        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        graph.add_node(NODE_NAME, ImageExportNode);
        graph.add_node_edge(CAMERA_DRIVER, NODE_NAME);
    }
}
