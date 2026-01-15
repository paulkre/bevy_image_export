use crate::{
    node::{ImageExportLabel, ImageExportNode},
    storage::save_image,
};
use bevy::{
    asset::RenderAssetUsages,
    ecs::system::{lifetimeless::SRes, SystemParamItem},
    prelude::*,
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        graph::CameraDriverLabel,
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_graph::RenderGraph,
        render_resource::{Buffer, BufferDescriptor, BufferUsages, Extent3d, MapMode},
        renderer::RenderDevice,
        texture::GpuImage,
        Render, RenderApp, RenderSystems,
    },
};
use futures::channel::oneshot;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use wgpu::PollType;

#[derive(Asset, Reflect, Clone, Default)]
pub struct ImageExportSource(pub Handle<Image>);

impl From<Handle<Image>> for ImageExportSource {
    fn from(value: Handle<Image>) -> Self {
        Self(value)
    }
}

#[derive(Component, ExtractComponent, Clone, Debug)]
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

impl RenderAsset for GpuImageExportSource {
    type SourceAsset = ImageExportSource;
    type Param = (SRes<RenderDevice>, SRes<RenderAssets<GpuImage>>);

    fn asset_usage(_: &Self::SourceAsset) -> RenderAssetUsages {
        RenderAssetUsages::default()
    }

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _asset_id: AssetId<Self::SourceAsset>,
        (device, images): &mut SystemParamItem<Self::Param>,
        _previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let Some(gpu_image) = images.get(&source_asset.0) else {
            return Err(PrepareAssetError::RetryNextUpdate(source_asset));
        };

        let size = gpu_image.texture.size();
        let format = &gpu_image.texture_format;
        let bytes_per_row =
            (size.width / format.block_dimensions().0) * format.block_copy_size(None).unwrap();
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
            source_handle: source_asset.0,
            source_size,
            bytes_per_row,
            padded_bytes_per_row,
        })
    }

    fn byte_len(_: &Self::SourceAsset) -> Option<usize> {
        None
    }
}

#[derive(Component, ExtractComponent, Clone, Debug)]
pub struct ImageExportStartFrame(u64);

impl Default for ImageExportSettings {
    fn default() -> Self {
        Self {
            output_dir: "out".into(),
            extension: "png".into(),
        }
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

#[derive(Component, ExtractComponent, Clone, Default, Debug)]
#[require(ImageExportSettings)]
pub struct ImageExport(pub Handle<ImageExportSource>);

#[derive(Default, Clone, Resource)]
pub struct ExportThreads {
    count: Arc<AtomicUsize>,
}

impl ExportThreads {
    /// Returns the number of threads currently running.
    pub fn thread_count(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }

    /// Checks if all threads have finished.
    pub fn is_finished(&self) -> bool {
        self.thread_count() == 0
    }

    /// Blocks the main thread until all frames have been saved successfully.
    pub fn finish(&self) {
        while !self.is_finished() {
            std::thread::sleep(std::time::Duration::from_secs_f32(0.25));
        }
    }

    pub(crate) fn report_thread_started(&self) {
        self.count.fetch_add(1, Ordering::SeqCst);
    }
}

fn save_buffer_to_disk(
    export_bundles: Query<(&ImageExport, &ImageExportSettings, &ImageExportStartFrame)>,
    sources: Res<RenderAssets<GpuImageExportSource>>,
    render_device: Res<RenderDevice>,
    export_threads: Res<ExportThreads>,
    mut frame_id: Local<u64>,
) {
    *frame_id = frame_id.wrapping_add(1);
    for (export, settings, start_frame) in &export_bundles {
        if let Some(gpu_source) = sources.get(&export.0) {
            let image_bytes = {
                let slice = gpu_source.buffer.slice(..);

                {
                    let (mapping_tx, mapping_rx) = oneshot::channel();

                    render_device.map_buffer(&slice, MapMode::Read, move |res| {
                        mapping_tx.send(res).unwrap();
                    });

                    if render_device
                        .poll(PollType::Wait {
                            submission_index: None,
                            timeout: None,
                        })
                        .is_err()
                    {
                        break;
                    }

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

            export_threads.report_thread_started();
            std::thread::spawn(move || {
                if let Err(err) = save_image(
                    &settings.output_dir,
                    &settings.extension,
                    image_bytes,
                    bytes_per_row,
                    padded_bytes_per_row,
                    source_size.width,
                    source_size.height,
                    frame_id,
                ) {
                    error!({ error = %err }, "failed saving image to disk");
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
    ImageExportSetup,
}

impl Plugin for ImageExportPlugin {
    fn build(&self, app: &mut App) {
        use ImageExportSystems::*;

        app.configure_sets(PostUpdate, ImageExportSetup)
            .register_type::<ImageExportSource>()
            .init_asset::<ImageExportSource>()
            .register_asset_reflect::<ImageExportSource>()
            .add_plugins((
                RenderAssetPlugin::<GpuImageExportSource>::default(),
                ExtractComponentPlugin::<ImageExport>::default(),
                ExtractComponentPlugin::<ImageExportSettings>::default(),
                ExtractComponentPlugin::<ImageExportStartFrame>::default(),
            ))
            .add_systems(PostUpdate, setup_exporters.in_set(ImageExportSetup));

        let render_app = app.sub_app_mut(RenderApp);

        render_app
            .insert_resource(self.threads.clone())
            .add_systems(
                Render,
                save_buffer_to_disk
                    .after(RenderSystems::Render)
                    .before(RenderSystems::Cleanup),
            );

        let mut graph = render_app
            .world_mut()
            .get_resource_mut::<RenderGraph>()
            .unwrap();

        graph.add_node(ImageExportLabel, ImageExportNode);
        graph.add_node_edge(CameraDriverLabel, ImageExportLabel);
    }
}
