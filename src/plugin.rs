use std::sync::{Arc, Mutex};

use crate::ecs::update_frame_id;

use super::{
    ecs::{export_image, extract_image_export_tasks, setup_export_data},
    node::{ImageExportNode, NODE_NAME},
};
use bevy::{
    prelude::*,
    render::{
        camera::CameraUpdateSystem, main_graph::node::CAMERA_DRIVER, render_graph::RenderGraph,
        RenderApp, RenderSet,
    },
};

#[derive(Default, Clone, Resource)]
pub struct ExportThreads {
    pub count: Arc<Mutex<u32>>,
}

impl ExportThreads {
    pub fn finish(&self) {
        while *self.count.lock().unwrap() > 0 {
            std::thread::sleep(std::time::Duration::from_secs_f32(0.25));
        }
    }
}

/// Plugin enabling the generation of image sequences.
#[derive(Default)]
pub struct ImageExportPlugin {
    pub threads: ExportThreads,
}

impl Plugin for ImageExportPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            setup_export_data
                .in_base_set(CoreSet::PostUpdate)
                .after(CameraUpdateSystem),
        )
        .add_system(update_frame_id.in_base_set(CoreSet::PostUpdate));

        let render_app = app.sub_app_mut(RenderApp);

        render_app.insert_resource(self.threads.clone());

        render_app
            .add_system(extract_image_export_tasks.in_schedule(ExtractSchedule))
            .add_system(export_image.in_set(RenderSet::RenderFlush));

        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        graph.add_node(NODE_NAME, ImageExportNode::default());
        graph.add_node_edge(CAMERA_DRIVER, NODE_NAME);
    }
}
