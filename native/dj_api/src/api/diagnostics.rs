//! Diagnostics endpoints used to check that the bridge is alive.

/// Versions of the native crates, shown in the app's "About" panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeVersions {
    pub domain: String,
    pub engine: String,
    pub infra: String,
}

/// Initialises the bridge runtime. Called once by the generated Dart code.
#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    flutter_rust_bridge::setup_default_user_utils();
}

/// Health check of the bridge: always answers `"pong"`.
#[flutter_rust_bridge::frb(sync)]
pub fn ping() -> String {
    "pong".to_owned()
}

/// Returns the versions of the native crates.
#[flutter_rust_bridge::frb(sync)]
pub fn native_versions() -> NativeVersions {
    NativeVersions {
        domain: dj_domain::VERSION.to_owned(),
        engine: dj_engine::VERSION.to_owned(),
        infra: dj_infra::VERSION.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{native_versions, ping};

    #[test]
    fn ping_answers_pong() {
        assert_eq!(ping(), "pong");
    }

    #[test]
    fn native_versions_are_filled() {
        let versions = native_versions();
        assert_ne!(versions.domain, "");
        assert_ne!(versions.engine, "");
        assert_ne!(versions.infra, "");
    }
}
