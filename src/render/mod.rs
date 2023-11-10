use cgmath::Vector3;

pub mod render;
mod view_data;
pub mod viewer;
pub mod view_core;

pub(crate) struct BBox {
    min: Vector3<f32>,
    max: Vector3<f32>,
}

impl Default for BBox {
    fn default() -> Self {
        Self {
            min: Vector3::new(f32::MAX, f32::MAX, f32::MAX),
            max: Vector3::new(f32::MIN, f32::MIN, f32::MIN),
        }
    }
}

impl BBox {
    #[inline]
    fn merge(&mut self, point: &Vector3<f32>) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }

    #[inline]
    fn merge_box(&mut self, other: &BBox) {
        self.merge(&other.min);
        self.merge(&other.max);
    }
}