use crate::{
    chat::PlayerSnapshot,
    entity_manager::{CefEntity, MODEL_HEIGHT, MODEL_WIDTH},
    error::*,
};
use classicube_sys::{Camera, Entities, LocalPlayer, RayTracer, Vec3, ENTITIES_SELF_ID};
use nalgebra::*;
use ncollide3d::{query::*, shape::*};

pub fn vec3_to_vector3(v: &Vec3) -> Vector3<f32> {
    Vector3::new(v.X, v.Y, v.Z)
}

// fn vector3_to_vec3(v: &Vector3<f32>) -> Vec3 {
//     Vec3::new(v.x, v.y, v.z)
// }

pub fn move_entity(entity: &mut CefEntity, player: &PlayerSnapshot) {
    let dir = Vec3::get_dir_vector(player.Yaw.to_radians(), player.Pitch.to_radians());

    entity.entity.Position.set(
        player.eye_position.X + dir.X,
        player.eye_position.Y + dir.Y,
        player.eye_position.Z + dir.Z,
    );

    // turn it to face the player
    entity.entity.RotY = player.Yaw + 180f32;
    entity.entity.RotX = 360f32 - player.Pitch;
}

pub fn get_camera_trace() -> Option<RayTracer> {
    let camera = unsafe { &*Camera.Active };
    let get_picked_block = camera.GetPickedBlock.unwrap();
    let mut ray_tracer = unsafe { std::mem::zeroed() };

    let entity_ptr = unsafe { Entities.List[ENTITIES_SELF_ID as usize] };
    let local_player = entity_ptr as *mut LocalPlayer;
    let local_player = unsafe { &mut *local_player };

    let old_reach_distance = local_player.ReachDistance;
    if local_player.ReachDistance < 32.0 {
        local_player.ReachDistance = 32.0;
    }
    unsafe {
        get_picked_block(&mut ray_tracer);
    }
    local_player.ReachDistance = old_reach_distance;

    // debug!("{:#?}", ray_tracer);
    if ray_tracer.Valid != 0 {
        Some(ray_tracer)
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
pub fn get_click_coords(
    eye_position: Vec3,
    entity_pos: Vec3,
    player_pitch: f32,
    player_yaw: f32,
    entity_pitch: f32,
    entity_yaw: f32,
    entity_scale: Vec3,
    browser_width: u32,
    browser_height: u32,
) -> Result<Option<(f32, f32)>> {
    fn intersect(
        eye_pos: Point3<f32>,
        [aim_pitch, aim_yaw]: [f32; 2],
        screen_pos: Point3<f32>,
        [screen_pitch, screen_yaw]: [f32; 2],
    ) -> Option<(Ray<f32>, Plane<f32>, RayIntersection<f32>)> {
        // when angles 0 0, aiming towards -z
        let normal = -Vector3::<f32>::z_axis();

        let aim_dir =
            Rotation3::from_euler_angles(-aim_pitch.to_radians(), -aim_yaw.to_radians(), 0.0)
                .transform_vector(&normal);

        // positive pitch is clockwise on the -x axis
        // positive yaw is clockwise on the -y axis
        let rot = UnitQuaternion::from_euler_angles(
            -screen_pitch.to_radians(),
            -screen_yaw.to_radians(),
            0.0,
        );
        let iso = Isometry3::from_parts(screen_pos.coords.into(), rot);

        let ray = Ray::new(eye_pos, aim_dir);
        let plane = Plane::new(normal);
        if let Some(intersection) = plane.toi_and_normal_with_ray(&iso, &ray, 32.0, true) {
            if intersection.toi == 0.0 {
                // 0 if aiming from wrong side
                None
            } else {
                Some((ray, plane, intersection))
            }
        } else {
            None
        }
    }

    let eye_pos = vec3_to_vector3(&eye_position);
    let screen_pos = vec3_to_vector3(&entity_pos);

    if let Some((ray, _plane, intersection)) = intersect(
        eye_pos.into(),
        [player_pitch, player_yaw],
        screen_pos.into(),
        [entity_pitch, entity_yaw],
    ) {
        let intersection_point = ray.point_at(intersection.toi).coords;

        let forward = intersection.normal;

        let tmp = Vector3::y();
        let right = Vector3::cross(&forward, &tmp);
        let right = right.normalize();
        let up = Vector3::cross(&right, &forward);
        let up = up.normalize();
        let right = -right;

        let width = entity_scale.X * MODEL_WIDTH as f32;
        let height = entity_scale.Y * MODEL_HEIGHT as f32;

        let top_left = screen_pos - 0.5 * right * width + up * height;

        let diff = intersection_point - top_left;
        let x = diff.dot(&right) / width;
        let y = -(diff.dot(&up) / height);

        if x < 0.0 || x > 1.0 || y < 0.0 || y > 1.0 {
            return Err("not looking at a screen".into());
        }

        let (x, y) = (x * browser_width as f32, y * browser_height as f32);

        Ok(Some((x, y)))
    } else {
        Ok(None)
    }
}
