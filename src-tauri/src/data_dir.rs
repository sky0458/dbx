use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
pub(crate) const PORTABLE_MARKER: &str = "portable.dbx";
#[cfg(target_os = "windows")]
pub(crate) const FIXED_WEBVIEW2_MARKER: &str = "fixed-webview2.dbx";
#[cfg(target_os = "windows")]
pub(crate) const MULTI_USER_MARKER: &str = "multi-user.dbx";
#[cfg(target_os = "windows")]
const INSTALLER_MARKER: &str = "uninstall.exe";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataDirMode {
    Default,
    EnvOverride,
    Portable { exe_dir: PathBuf },
    SharedPortable { exe_dir: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDirResolution {
    pub data_dir: PathBuf,
    pub default_data_dir: PathBuf,
    pub mode: DataDirMode,
    portable_data_dir: Option<PathBuf>,
}

impl DataDirResolution {
    pub fn uses_custom_data_dir(&self) -> bool {
        matches!(self.mode, DataDirMode::EnvOverride | DataDirMode::Portable { .. })
    }

    pub fn is_portable_mode(&self) -> bool {
        matches!(self.mode, DataDirMode::Portable { .. } | DataDirMode::SharedPortable { .. })
    }
}

pub fn resolve_data_dir_with_mode(default_app_data_dir: PathBuf) -> DataDirResolution {
    let env_data_dir = std::env::var_os("DBX_DATA_DIR").filter(|value| !value.is_empty()).map(PathBuf::from);

    #[cfg(target_os = "windows")]
    let exe_dir = current_exe_dir();
    #[cfg(not(target_os = "windows"))]
    let exe_dir = None;

    let portable_marker_exists = exe_dir.as_deref().is_some_and(portable_marker_exists);
    let multi_user_marker_exists = exe_dir.as_deref().is_some_and(multi_user_marker_exists);
    let installer_marker_exists = exe_dir.as_deref().is_some_and(installer_marker_exists);

    resolve_data_dir_from_inputs(
        default_app_data_dir,
        exe_dir,
        portable_marker_exists,
        multi_user_marker_exists,
        installer_marker_exists,
        env_data_dir,
    )
}

pub fn alternative_data_dir(resolution: &DataDirResolution) -> Option<PathBuf> {
    match &resolution.mode {
        DataDirMode::Portable { .. } => Some(resolution.default_data_dir.clone()),
        DataDirMode::SharedPortable { .. } => None,
        DataDirMode::Default => resolution.portable_data_dir.clone(),
        DataDirMode::EnvOverride => None,
    }
}

pub fn is_portable_mode() -> bool {
    resolve_data_dir_with_mode(PathBuf::new()).is_portable_mode()
}

pub fn is_fixed_webview2_portable_mode() -> bool {
    #[cfg(target_os = "windows")]
    {
        return current_exe_dir().is_some_and(|exe_dir| {
            fixed_webview2_portable_from_inputs(
                exe_dir.join(PORTABLE_MARKER).is_file(),
                exe_dir.join(FIXED_WEBVIEW2_MARKER).is_file(),
                exe_dir.join(INSTALLER_MARKER).is_file(),
            )
        });
    }

    #[cfg(not(target_os = "windows"))]
    false
}

fn fixed_webview2_portable_from_inputs(
    portable_marker_exists: bool,
    fixed_marker_exists: bool,
    installer_marker_exists: bool,
) -> bool {
    portable_marker_exists && fixed_marker_exists && !installer_marker_exists
}

#[cfg(target_os = "windows")]
fn current_exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|path| path.parent().map(Path::to_path_buf))
}

#[cfg(target_os = "windows")]
fn portable_marker_exists(exe_dir: &Path) -> bool {
    exe_dir.join(PORTABLE_MARKER).is_file()
}

#[cfg(not(target_os = "windows"))]
fn portable_marker_exists(_exe_dir: &Path) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn multi_user_marker_exists(exe_dir: &Path) -> bool {
    exe_dir.join(MULTI_USER_MARKER).is_file()
}

#[cfg(not(target_os = "windows"))]
fn multi_user_marker_exists(_exe_dir: &Path) -> bool {
    false
}

#[cfg(target_os = "windows")]
fn installer_marker_exists(exe_dir: &Path) -> bool {
    exe_dir.join(INSTALLER_MARKER).is_file()
}

#[cfg(not(target_os = "windows"))]
fn installer_marker_exists(_exe_dir: &Path) -> bool {
    false
}

fn resolve_data_dir_from_inputs(
    default_app_data_dir: PathBuf,
    exe_dir: Option<PathBuf>,
    portable_marker_exists: bool,
    multi_user_marker_exists: bool,
    installer_marker_exists: bool,
    env_data_dir: Option<PathBuf>,
) -> DataDirResolution {
    let portable_data_dir = exe_dir.as_ref().filter(|_| portable_marker_exists).map(|dir| dir.join("data"));

    if let Some(env_dir) = env_data_dir {
        return DataDirResolution {
            data_dir: env_dir,
            default_data_dir: default_app_data_dir,
            mode: DataDirMode::EnvOverride,
            portable_data_dir,
        };
    }

    if portable_marker_exists && !installer_marker_exists {
        if let Some(exe_dir) = exe_dir {
            if multi_user_marker_exists {
                return DataDirResolution {
                    data_dir: default_app_data_dir.clone(),
                    default_data_dir: default_app_data_dir,
                    mode: DataDirMode::SharedPortable { exe_dir },
                    portable_data_dir,
                };
            }

            return DataDirResolution {
                data_dir: exe_dir.join("data"),
                default_data_dir: default_app_data_dir,
                mode: DataDirMode::Portable { exe_dir },
                portable_data_dir,
            };
        }
    }

    DataDirResolution {
        data_dir: default_app_data_dir.clone(),
        default_data_dir: default_app_data_dir,
        mode: DataDirMode::Default,
        portable_data_dir,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{alternative_data_dir, fixed_webview2_portable_from_inputs, resolve_data_dir_from_inputs, DataDirMode};

    #[test]
    fn uses_portable_data_dir_when_marker_exists_without_installer_marker() {
        let default_dir = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\com.dbx.app");
        let exe_dir = PathBuf::from(r"D:\Apps\DBX");

        let resolution = resolve_data_dir_from_inputs(default_dir, Some(exe_dir.clone()), true, false, false, None);

        assert_eq!(resolution.data_dir, exe_dir.join("data"));
        assert_eq!(resolution.mode, DataDirMode::Portable { exe_dir });
        assert!(resolution.uses_custom_data_dir());
        assert!(resolution.is_portable_mode());
    }

    #[test]
    fn installer_marker_keeps_installed_mode_even_when_portable_marker_exists() {
        let default_dir = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\com.dbx.app");
        let exe_dir = PathBuf::from(r"C:\Program Files\DBX");

        let resolution = resolve_data_dir_from_inputs(default_dir.clone(), Some(exe_dir), true, false, true, None);

        assert_eq!(resolution.data_dir, default_dir);
        assert_eq!(resolution.mode, DataDirMode::Default);
        assert!(!resolution.uses_custom_data_dir());
        assert!(!resolution.is_portable_mode());
    }

    #[test]
    fn env_override_wins_over_installer_and_portable_markers() {
        let default_dir = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\com.dbx.app");
        let exe_dir = PathBuf::from(r"C:\Program Files\DBX");
        let env_dir = PathBuf::from(r"E:\DBXData");

        let resolution =
            resolve_data_dir_from_inputs(default_dir, Some(exe_dir), true, true, true, Some(env_dir.clone()));

        assert_eq!(resolution.data_dir, env_dir);
        assert_eq!(resolution.mode, DataDirMode::EnvOverride);
        assert!(resolution.uses_custom_data_dir());
        assert!(!resolution.is_portable_mode());
    }

    #[test]
    fn portable_mode_can_import_from_default_data_dir() {
        let default_dir = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\com.dbx.app");
        let exe_dir = PathBuf::from(r"D:\Apps\DBX");

        let resolution = resolve_data_dir_from_inputs(default_dir.clone(), Some(exe_dir), true, false, false, None);

        assert_eq!(alternative_data_dir(&resolution), Some(default_dir));
    }

    #[test]
    fn shared_portable_mode_keeps_application_data_per_windows_user() {
        let default_dir = PathBuf::from(r"C:\Users\alice\AppData\Roaming\com.dbx.app");
        let exe_dir = PathBuf::from(r"D:\SharedApps\DBX");

        let resolution =
            resolve_data_dir_from_inputs(default_dir.clone(), Some(exe_dir.clone()), true, true, false, None);

        assert_eq!(resolution.data_dir, default_dir);
        assert_eq!(resolution.mode, DataDirMode::SharedPortable { exe_dir });
        assert!(!resolution.uses_custom_data_dir());
        assert!(resolution.is_portable_mode());
        assert_eq!(alternative_data_dir(&resolution), None);
    }

    #[test]
    fn installed_mode_can_import_from_leftover_portable_data_dir() {
        let default_dir = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\com.dbx.app");
        let exe_dir = PathBuf::from(r"C:\Program Files\DBX");

        let resolution = resolve_data_dir_from_inputs(default_dir, Some(exe_dir.clone()), true, false, true, None);

        assert_eq!(alternative_data_dir(&resolution), Some(exe_dir.join("data")));
    }

    #[test]
    fn env_override_does_not_import_from_implicit_alternative_dir() {
        let default_dir = PathBuf::from(r"C:\Users\Administrator\AppData\Roaming\com.dbx.app");
        let exe_dir = PathBuf::from(r"D:\Apps\DBX");

        let resolution = resolve_data_dir_from_inputs(
            default_dir,
            Some(exe_dir),
            true,
            true,
            false,
            Some(PathBuf::from(r"E:\DBXData")),
        );

        assert_eq!(alternative_data_dir(&resolution), None);
    }

    #[test]
    fn fixed_webview2_mode_requires_both_portable_markers_and_no_installer() {
        assert!(fixed_webview2_portable_from_inputs(true, true, false));
        assert!(!fixed_webview2_portable_from_inputs(false, true, false));
        assert!(!fixed_webview2_portable_from_inputs(true, false, false));
        assert!(!fixed_webview2_portable_from_inputs(true, true, true));
    }
}
