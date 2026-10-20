//! Shared GPU state for every window in the application.
//!
//! One device and queue are used for the editor and all output windows. This
//! is the single-machine topology from the RFC; distributed outputs will use
//! the same renderer through a different `OutputSink`.

use anyhow::{Context, Result};

pub struct Gpu {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_info: wgpu::AdapterInfo,
}

impl Gpu {
    pub fn new(instance: wgpu::Instance) -> Result<Self> {
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: true,
        }))
        .context("no compatible GPU adapter found")?;

        let adapter_info = adapter.get_info();
        log::info!(
            "GPU adapter: {} ({:?}, {:?})",
            adapter_info.name,
            adapter_info.backend,
            adapter_info.device_type
        );

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("lpmt-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .context("failed to create GPU device")?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            adapter_info,
        })
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }
}
