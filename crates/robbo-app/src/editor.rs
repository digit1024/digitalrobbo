use bevy::prelude::*;

use crate::app_state::AppState;

/// Stretch-goal level editor stub (M7).
#[derive(Resource, Default)]
pub struct EditorState {
    pub active: bool,
    pub brush: EditorBrush,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum EditorBrush {
    #[default]
    Empty,
    Wall,
    Screw,
    Robbo,
}

pub fn editor_toggle(keys: Res<ButtonInput<KeyCode>>, mut editor: ResMut<EditorState>) {
    if keys.just_pressed(KeyCode::F9) {
        editor.active = !editor.active;
    }
}

pub fn editor_ui(editor: Res<EditorState>, state: Res<State<AppState>>) {
    if *state.get() == AppState::Playing && editor.active {
        // Placeholder: full editor in M7 stretch
    }
}
