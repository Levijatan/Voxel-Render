use bitvec::{array::BitArray, order::LocalBits, field::BitField, view::BitView};
use std::ops::Range;
use geom::voxel;
use geom::util;

pub type MetaU64 = Meta<u32, [u64; geom::chunk::VOXELS_IN_CHUNK/64]>;

impl Default for MetaU64 {
    fn default() -> Self {
        Self::new([1; geom::chunk::VOXELS_IN_CHUNK/64])
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Meta<O, V>
    where O: Copy + Clone, V: BitView + Sized
{
    //0-5 transparency, 6 visibility
    visibility: BitArray<LocalBits, [u8; 1]>,
    voxel_visibility: BitArray<LocalBits, V>,
    render_offset: Option<O>,
    pub render_amount: u16,
}

impl<O, V> Meta<O, V>
    where O: Copy + Clone, V: BitView + Sized
{
    pub fn new(data: V) -> Self {
        Self {
            visibility: BitArray::new([0; 1]),
            voxel_visibility: BitArray::new(data),
            render_offset: None,
            render_amount: 0,
        }
    }

    pub fn has_render_offset(&self) -> bool {
        self.render_offset.is_some()
    }

    pub fn set_render_offset(&mut self, value: Option<O>) {
        self.render_offset = value;
    }

    pub fn render_offset(&self) -> Option<O> {
        self.render_offset
    }
}

impl<O: Copy + Clone, V: BitView + Sized> geom::chunk::Meta for Meta<O, V>
{
    fn set_transparency(&mut self, value: u8) {
        assert!(value < 64, "Max val allowed 63");
        self.visibility[..6].store(value);
    }

    fn set_visibilty(&mut self, value: bool) {
        self.visibility.set(6, value)
    }

    fn is_visible(&self) -> bool {
        *self.visibility.get(6).unwrap()
    }

    fn voxel_set_range(&mut self, range: Range<usize>, value: bool) {
        self.voxel_visibility[range].set_all(value);
    }

    #[allow(clippy::cast_sign_loss)]
    fn voxel_is_visible(&self, p: voxel::Position) -> bool {
        let idx = util::calc_voxel_idx(p.x(), p.y(), p.z());
        assert!(idx >= 0, "idx has to be a positive number");
        *self.voxel_visibility.get(idx as usize).unwrap()
    }
}