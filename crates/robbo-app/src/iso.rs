//! Isometric projection plugin — enable with `ActiveProjection.use_isometric = true`.
use bevy::prelude::*;

use crate::projection::ActiveProjection;

pub fn toggle_isometric(keys: Res<ButtonInput<KeyCode>>, mut projection: ResMut<ActiveProjection>) {
    if keys.just_pressed(KeyCode::F8) {
        projection.use_isometric = !projection.use_isometric;
    }
}
