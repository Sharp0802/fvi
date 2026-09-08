use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::{instrument, warn};
use wgpu::*;

use super::adapter::select_adapter;
use super::{DevicePref, RenderConfig};
use crate::error::InitError;

/// A rendering device.
#[derive(Debug)]
pub struct RenderDevice {
    name: String,
    adapter: Adapter,
    device: Device,
    queue: Queue,
    is_lost: Arc<AtomicBool>,
}

impl RenderDevice {
    #[instrument]
    pub(crate) async fn new(
        instance: &Instance,
        surface: &Surface<'_>,
        pref: &DevicePref,
        conf: &RenderConfig,
    ) -> Result<Self, InitError> {
        let adapter = select_adapter(instance, surface, pref, conf).await?;
        let name = adapter.get_info().name;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("RenderDevice.device"),
                required_features: conf.features,
                required_limits: conf.limits.clone(),
                experimental_features: ExperimentalFeatures::disabled(),
                memory_hints: conf.memory_hints.clone(),
                trace: Trace::Off,
            })
            .await?;

        let is_lost = Arc::new(AtomicBool::new(false));

        let is_lost_clone = is_lost.clone();
        device.set_device_lost_callback(move |reason, message| {
            is_lost_clone.store(true, Ordering::Relaxed);
            warn!(reason = ?reason, message = message, "device lost");
        });

        Ok(Self {
            name,
            adapter,
            device,
            queue,
            is_lost,
        })
    }

    #[must_use]
    pub(crate) fn is_lost(&self) -> bool {
        self.is_lost.load(Ordering::Relaxed)
    }

    /// Returns the name of corresponding adapter.
    #[must_use]
    pub const fn name(&self) -> &str {
        self.name.as_str()
    }
}

impl Deref for RenderDevice {
    type Target = Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

impl AsRef<Adapter> for RenderDevice {
    fn as_ref(&self) -> &Adapter {
        &self.adapter
    }
}

impl AsRef<Device> for RenderDevice {
    fn as_ref(&self) -> &Device {
        &self.device
    }
}

impl AsRef<Queue> for RenderDevice {
    fn as_ref(&self) -> &Queue {
        &self.queue
    }
}
