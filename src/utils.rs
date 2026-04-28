use std::process::exit;
use std::sync::Arc;

pub enum SurfaceError {
    Timeout,
    Lost,
    Occluded,
    Outdated,
    Validation,
}

pub fn try_create_surface(
    window: Arc<winit::window::Window>,
) -> (wgpu::Instance, wgpu::Surface<'static>) {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        flags: wgpu::InstanceFlags::default(),
        memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
        backend_options: wgpu::BackendOptions::default(),
        display: None,
    });
    let surface_hopefully = instance.create_surface(window.clone());
    match surface_hopefully {
        Err(e) => {
            eprint!("failed to create surface: {e}");
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::GL,
                flags: wgpu::InstanceFlags::default(),
                memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
                backend_options: wgpu::BackendOptions::default(),
                display: None,
            });
            let surface = instance.create_surface(window.clone()).unwrap();
            return (instance, surface);
        }
        Ok(surface) => (instance, surface),
    }
}
