/// The wgpu renderer for the game canvas.
///
/// Consumes RenderSnapshot from the core simulation (stays in Rust,
/// never crosses the WASM bridge). Handles sprite batching, camera,
/// effects, and canvas output.
///
/// v1 uses sprite-swap animation. Bone rigs are post-v1.
#[derive(Default)]
pub struct Renderer {
    // TODO: wgpu device, queue, pipeline, sprite atlas, camera
}

impl Renderer {
    /// Initialize the renderer with a canvas element.
    pub fn new() -> Self {
        // TODO: wgpu initialization
        Self {}
    }

    /// Render one frame. Called from the game loop after tick().
    /// `alpha` is the interpolation fraction between sim ticks.
    pub fn render(&mut self, _alpha: f32) {
        // TODO: read RenderSnapshot, interpolate positions, batch sprites, draw
    }
}
