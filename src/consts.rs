//Voxel string ids

pub const OPAQUE_VOXEL: &str = "opaque";
pub const TRANSPARENT_VOXEL: &str = "transparent";

pub const INVALID_VOXEL_ID: u64 = 0;

pub const SCREEN_WIDTH: u32 = 2560;
pub const SCREEN_HEIGHT: u32 = 1440;

pub const VOXEL_SIZE: f32 = 2.0;
pub const CHUNK_SIZE_F32: f32 = 16.0;
pub const CHUNK_SIZE_I64: i64 = CHUNK_SIZE_F32 as i64;
pub const CHUNK_SIZE_USIZE: usize = CHUNK_SIZE_F32 as usize;
pub const CHUNK_SIZE_U64: u64 = CHUNK_SIZE_F32 as u64;

pub const RENDER_RADIUS: u32 = 5;

pub const TICK_PER_SEC: f32 = 20.0;
pub const TICK_STEP: f32 = 1.0 / TICK_PER_SEC;
