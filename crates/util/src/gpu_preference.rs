//! Which GPU an app asks to run on: a desktop entry with `PrefersNonDefaultGPU=true` wants the
//! one the system does not render with by default, the discrete GPU of a hybrid laptop.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GpuPreference {
    #[default]
    Default,
    NonDefault,
}

impl GpuPreference {
    pub const DESKTOP_ENTRY_KEY: &str = "PrefersNonDefaultGPU";

    /// Reads the value of [`Self::DESKTOP_ENTRY_KEY`].
    pub fn from_desktop_entry(value: Option<&str>) -> Self {
        match value {
            Some("true") => Self::NonDefault,
            _ => Self::Default,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_true_prefers_the_non_default_gpu() {
        assert_eq!(
            GpuPreference::from_desktop_entry(Some("true")),
            GpuPreference::NonDefault
        );
        assert_eq!(
            GpuPreference::from_desktop_entry(Some("false")),
            GpuPreference::Default
        );
        assert_eq!(
            GpuPreference::from_desktop_entry(None),
            GpuPreference::Default
        );
    }
}
