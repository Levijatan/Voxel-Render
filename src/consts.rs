//Voxel string ids

pub const OPAQUE_VOXEL: &str = "opaque";
pub const TRANSPARENT_VOXEL: &str = "transparent";

pub const SCREEN_WIDTH: u32 = 2560;
pub const SCREEN_HEIGHT: u32 = 1440;

pub const VOXEL_SIZE: f32 = 2.0;
pub const CHUNK_SIZE_F32: f32 = 16.0;
pub const CHUNK_SIZE_U32: u32 = CHUNK_SIZE_F32 as u32;
pub const CHUNK_SIZE_I32: i32 = CHUNK_SIZE_F32 as i32;
pub const CHUNK_SIZE_USIZE: usize = CHUNK_SIZE_F32 as usize;

pub const RENDER_RADIUS: u8 = 7;

pub const TICK_PER_SEC: f32 = 20.0;
pub const TICK_STEP: f32 = 1.0 / TICK_PER_SEC;