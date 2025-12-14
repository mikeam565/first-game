use bevy::prelude::*;

/// Unified render state for spatial entities (terrain chunks, grass tiles, etc.)
/// Differentiates between visibility state and entity lifecycle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderState {
    /// Entity is fully rendered and visible
    Visible,
    /// Entity exists but is hidden (outside camera frustum), OR
    /// no entity exists yet for this grid position (will be spawned when visible)
    Hidden,
    /// Entity is being generated asynchronously
    Pending,
}

/// Marker component for entities that have been hidden due to frustum culling
/// This allows efficient querying of hidden entities
#[derive(Component)]
pub struct FrustumHidden;
