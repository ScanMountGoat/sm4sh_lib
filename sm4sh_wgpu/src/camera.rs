use glam::{Vec3, Vec4, Vec4Swizzles};

/// Frame the region defined by `bounding_sphere`
/// assuming a camera pointing toward the positive z-axis.
pub fn frame_bounding_sphere_translation(bounding_sphere: Vec4, fov_y: f32, aspect: f32) -> Vec3 {
    let center = bounding_sphere.xyz();
    let radius = bounding_sphere.w;

    // Find the base of the triangle based on FOV and model bounds.
    // Take the max to frame both horizontally and vertically.
    // TODO: Take near clip into account.
    let fov_x = 2.0 * (aspect * (fov_y * 0.5).tan()).atan();
    let distance_y = radius / (fov_y * 0.5).tan();
    let distance_x = radius / (fov_x * 0.5).tan();
    let distance = distance_x.max(distance_y);

    // This distance should be from the closest object depth.
    let closest_z = center.z - radius;
    glam::vec3(-center.x, -center.y, closest_z - distance)
}
