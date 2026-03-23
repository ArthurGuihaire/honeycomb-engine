pub enum SurfaceError {
    Outdated,
    OutOfMemory,
    Other(wgpu::SurfaceError),
}
