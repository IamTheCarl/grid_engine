#![cfg_attr(target_arch = "spirv", no_std, feature(register_attr), register_attr(spirv))]

// Use the spirv macros if we're not already building on a spirv platform.
#[cfg(not(target_arch = "spirv"))]
use spirv_std::macros::spirv;

use glam::{vec4, Vec3, Vec4};

#[spirv(vertex)]
pub fn main_vs(a_position: Vec3, a_color: Vec3, v_color: &mut Vec3, #[spirv(position, invariant)] out_pos: &mut Vec4) {
    *v_color = a_color;
    *out_pos = vec4(a_position.x, a_position.y, a_position.z, 1.0);
}

#[spirv(fragment)]
pub fn main_fs(v_color: Vec3, f_color: &mut Vec4) {
    *f_color = vec4(v_color.x, v_color.y, v_color.z, 1.0);
}
