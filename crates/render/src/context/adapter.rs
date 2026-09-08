use super::{BACKENDS, DevicePref, RenderConfig};
use crate::error::InitError;
use tracing::{debug, info, instrument, trace, warn};
use wgpu::*;

fn is_compatible(adapter: &Adapter, surface: &Surface, config: &RenderConfig) -> bool {
    let name = &adapter.get_info().name;
    let mut is_compat = true;

    let limits = adapter.limits();
    let limits_mask = config.limits.clone().or_better_values_from(&limits);
    limits.check_limits_with_fail_fn(&limits_mask, false, |limit_name, actual, req| {
        is_compat = false;
        trace!("{name}: {limit_name}={actual}, {req} required");
    });

    let feat_diff = config.features.difference(adapter.features());
    if !feat_diff.is_empty() {
        is_compat = false;

        for (feat_name, _flag) in feat_diff.iter_names() {
            trace!("{name}: {feat_name} not supported");
        }
    }

    let capabilities = surface.get_capabilities(adapter);

    let usage_diff = config.texture_usages.difference(capabilities.usages);
    if !usage_diff.is_empty() {
        is_compat = false;

        for (usage_name, _flag) in usage_diff.iter_names() {
            trace!("{name}: {usage_name} not supported");
        }
    }

    if capabilities.formats.is_empty() {
        is_compat = false;
        trace!("{name}: no compatible surface format");
    }

    if !adapter.is_surface_supported(surface) {
        is_compat = false;
        trace!("{name}: surface not supported");
    }

    is_compat
}

const fn score_device_type(ty: DeviceType, pref: &DevicePref) -> u32 {
    let (igpu, dgpu) = if pref.low_power { (4, 3) } else { (3, 4) };

    match ty {
        DeviceType::IntegratedGpu => igpu,
        DeviceType::DiscreteGpu => dgpu,
        DeviceType::VirtualGpu => 2,
        DeviceType::Cpu => 1,
        DeviceType::Other => 0,
    }
}

fn select_adapter_from(
    adapters: impl Iterator<Item = Adapter>,
    surface: &Surface,
    pref: &DevicePref,
    config: &RenderConfig,
) -> Option<Adapter> {
    let mut founds = Vec::new();
    for adapter in adapters {
        let info = adapter.get_info();
        let name = &info.name;
        if !is_compatible(&adapter, surface, config) {
            debug!("{name}: is not compatible");
            continue;
        }

        info!("{name}: found compatible adapter");
        let score = score_device_type(info.device_type, pref);
        founds.push((adapter, score));
    }
    if founds.is_empty() {
        warn!("no compatible adapter found");
        return None;
    }

    founds.sort_unstable_by_key(|(_, score)| *score);
    let (adapter, _score) = founds.pop().expect("founds.len() > 0");

    let name = adapter.get_info().name;
    info!("{name}: adapter selected");

    Some(adapter)
}

async fn select_cached_adapter(
    instance: &Instance,
    surface: &Surface<'_>,
    pref: &DevicePref,
    config: &RenderConfig,
) -> Option<Adapter> {
    let name = pref.name.as_ref().expect("adapter name not cached");
    info!("{name}: getting cached adapter...");

    let mut adapters = instance
        .enumerate_adapters(BACKENDS)
        .await
        .into_iter()
        .filter(|adapter| &adapter.get_info().name == name)
        .peekable();

    if adapters.peek().is_none() {
        warn!("{name}: cached adapter not found");
        return None;
    }

    let found = select_adapter_from(adapters, surface, pref, config);
    if found.is_none() {
        warn!("{name}: cached adapter not compatible");
    }

    found
}

#[instrument]
pub(super) async fn select_adapter(
    instance: &Instance,
    surface: &Surface<'_>,
    pref: &DevicePref,
    config: &RenderConfig,
) -> Result<Adapter, InitError> {
    if pref.name.is_some()
        && let Some(found) = select_cached_adapter(instance, surface, pref, config).await
    {
        return Ok(found);
    }

    let adapters = instance.enumerate_adapters(BACKENDS).await;
    if let Some(found) = select_adapter_from(adapters.into_iter(), surface, pref, config) {
        return Ok(found);
    }

    warn!("no compatible adapter found; requesting default...");

    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: if pref.low_power {
                PowerPreference::LowPower
            } else {
                PowerPreference::HighPerformance
            },
            force_fallback_adapter: false,
            compatible_surface: Some(surface),
            apply_limit_buckets: false,
        })
        .await?;

    let name = adapter.get_info().name;
    warn!("{name}: adapter found as fallback; may be incompatible");
    Ok(adapter)
}

#[instrument]
pub(super) async fn list_adapters(
    instance: &Instance,
    surface: &Surface<'_>,
    config: &RenderConfig,
) -> impl Iterator<Item = String> {
    instance
        .enumerate_adapters(BACKENDS)
        .await
        .into_iter()
        .filter(|adapter| is_compatible(adapter, surface, config))
        .map(|adapter| adapter.get_info().name)
}
