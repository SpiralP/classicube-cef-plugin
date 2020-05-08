use crate::{chat::PlayerSnapshot, entity_manager::CefEntity};
use classicube_sys::{Camera, Entities, LocalPlayer, RayTracer, Vec3, ENTITIES_SELF_ID};
use nalgebra::*;

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
