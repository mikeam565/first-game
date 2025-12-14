/// Unified render state for spatial entities (terrain chunks, grass tiles, etc.)
/// Differentiates between visibility state and entity lifecycle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RenderState {
    /// Entity is fully rendered and visible
    Visible,
    /// Entity is being generated asynchronously
    Pending,
}
