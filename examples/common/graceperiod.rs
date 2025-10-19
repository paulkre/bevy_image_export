use bevy::prelude::*;

/// A resource that keeps track of the current frame and the number of frames that have to pass
/// before the next frame is considered "graceful".
#[derive(Resource)]
pub struct GracefulFrameCount {
    frame: u32,
    grace_period_frames: u32,
}

impl GracefulFrameCount {
    pub fn new(grace_period_frames: u32) -> Self {
        Self {
            frame: 0,
            grace_period_frames,
        }
    }

    pub fn frame(&self) -> u32 {
        self.frame
    }
}

/// A plugin that adds a [`GracefulFrameCount`] resource to the app.
///
/// This is necessary because Bevy doesn't predictably start rendering on the first frame:
/// https://github.com/bevyengine/bevy/issues/21600
pub struct GracePeriodPlugin {
    grace_period_frames: u32,
}

impl Default for GracePeriodPlugin {
    fn default() -> Self {
        Self {
            grace_period_frames: 10,
        }
    }
}

impl Plugin for GracePeriodPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(GracefulFrameCount::new(self.grace_period_frames))
            .add_systems(PreUpdate, update_graceful_frame_count);
    }
}

fn update_graceful_frame_count(
    mut graceful_frame_count: ResMut<GracefulFrameCount>,
    mut frame: Local<u32>,
) {
    *frame = frame.wrapping_add(1);
    if *frame > graceful_frame_count.grace_period_frames {
        graceful_frame_count.frame = *frame - graceful_frame_count.grace_period_frames;
    }
}
