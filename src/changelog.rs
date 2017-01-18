//! Project Changelog

/// Release 0.2.0 (2017-01-18)
///
/// # Breaking changes
///
/// * `TargetInfo::target_vendor` changed signature to return `Option<&str>` instead of `&str`.
/// Non-nightly rustc doesn’t give the information about target vendor, so it is not available when
/// compiling with stable/beta rustc.
///
/// # Other changes
///
/// * Added `TargetInfo::target_cfg`. Can be used to emulate e.g. `#[cfg(unix)]` or
/// `#[cfg(windows)]`.
/// * Added `TargetInfo::target_cfg_value`. Can be used to extract more obscure target properties
/// such as `#[cfg(target_has_atomic = "64")]`. Note that many of these depend on rustc channel,
/// just like `target_vendor`.
pub mod r0_2_0 {}

/// Release 0.1.2 (2016-10-17)
///
/// * Now figures out target info from the rustc that is used to compile the library. This results
/// in less divergence between versions of rustc (i.e. when targets are added), but is not able to
/// provide target info for some targets on some hosts anymore. For example all `*-apple-ios`
/// targets are not available anymore on the linux host.
/// * `Error` implements `std::error::Error` trait now.
pub mod r0_1_2 {}
