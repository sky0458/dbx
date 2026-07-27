use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[cfg(target_os = "windows")]
use crate::data_dir::{FIXED_WEBVIEW2_MARKER, MULTI_USER_MARKER, PORTABLE_MARKER};

const FIXED_WEBVIEW2_RUNTIME_DIR: &str = "WebView2Runtime";
const FIXED_WEBVIEW2_USER_DATA_SUBDIR: &str = "FixedRuntime";
#[cfg(target_os = "windows")]
const FIXED_WEBVIEW2_EXECUTABLE: &str = "msedgewebview2.exe";
#[cfg(target_os = "windows")]
const FIXED_WEBVIEW2_FORCE_SANDBOX_ENV: &str = "DBX_WEBVIEW2_FORCE_SANDBOX";
#[cfg(target_os = "windows")]
const WEBVIEW2_NO_SANDBOX_ENV: &str = "DBX_WEBVIEW2_NO_SANDBOX";
#[cfg(target_os = "windows")]
const WEBVIEW2_BROWSER_EXECUTABLE_FOLDER_ENV: &str = "WEBVIEW2_BROWSER_EXECUTABLE_FOLDER";
#[cfg(target_os = "windows")]
const WEBVIEW2_USER_DATA_FOLDER_ENV: &str = "WEBVIEW2_USER_DATA_FOLDER";
#[cfg(target_os = "windows")]
const WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS_ENV: &str = "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS";

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixedRuntimePaths {
    browser_executable_folder: PathBuf,
    user_data_folder: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebView2Environment {
    browser_executable_folder: Option<OsString>,
    user_data_folder: Option<OsString>,
    additional_browser_arguments: Option<OsString>,
}

fn fixed_runtime_paths_from_inputs(
    exe_dir: Option<&Path>,
    portable_marker_exists: bool,
    fixed_marker_exists: bool,
    runtime_executable_exists: bool,
    multi_user_marker_exists: bool,
    per_user_data_root: Option<&Path>,
) -> Option<FixedRuntimePaths> {
    let exe_dir = exe_dir?;
    (portable_marker_exists && fixed_marker_exists && runtime_executable_exists).then(|| FixedRuntimePaths {
        browser_executable_folder: exe_dir.join(FIXED_WEBVIEW2_RUNTIME_DIR),
        user_data_folder: if multi_user_marker_exists {
            per_user_data_root
                .map(|root| root.join("com.dbx.app").join("WebView2").join(FIXED_WEBVIEW2_USER_DATA_SUBDIR))
                .unwrap_or_else(|| exe_dir.join("data").join("webview2"))
        } else {
            exe_dir.join("data").join("webview2")
        },
    })
}

fn per_user_local_data_root(local_app_data: Option<OsString>, user_profile: Option<OsString>) -> Option<PathBuf> {
    non_empty(local_app_data)
        .map(PathBuf::from)
        .or_else(|| non_empty(user_profile).map(PathBuf::from).map(|path| path.join("AppData").join("Local")))
}

fn non_empty(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

fn with_no_sandbox(arguments: Option<OsString>) -> OsString {
    let mut arguments = arguments.unwrap_or_default().to_string_lossy().into_owned();
    if !arguments.split_whitespace().any(|argument| argument == "--no-sandbox") {
        if !arguments.is_empty() {
            arguments.push(' ');
        }
        arguments.push_str("--no-sandbox");
    }
    OsString::from(arguments)
}

fn resolve_environment(
    fixed_runtime: Option<FixedRuntimePaths>,
    existing_browser_folder: Option<OsString>,
    existing_user_data_folder: Option<OsString>,
    existing_browser_arguments: Option<OsString>,
    force_sandbox: bool,
    request_no_sandbox: bool,
) -> WebView2Environment {
    let existing_browser_folder = non_empty(existing_browser_folder);
    let existing_user_data_folder = non_empty(existing_user_data_folder);
    let existing_browser_arguments = non_empty(existing_browser_arguments);
    let fixed_runtime_active = fixed_runtime.is_some();

    let (browser_executable_folder, user_data_folder) = match fixed_runtime {
        Some(paths) => (
            existing_browser_folder.or_else(|| Some(paths.browser_executable_folder.into_os_string())),
            existing_user_data_folder.or_else(|| Some(paths.user_data_folder.into_os_string())),
        ),
        None => (existing_browser_folder, existing_user_data_folder),
    };
    let should_disable_sandbox = !force_sandbox && (fixed_runtime_active || request_no_sandbox);
    let additional_browser_arguments = should_disable_sandbox
        .then(|| with_no_sandbox(existing_browser_arguments.clone()))
        .or(existing_browser_arguments);

    WebView2Environment { browser_executable_folder, user_data_folder, additional_browser_arguments }
}

#[cfg(target_os = "windows")]
pub(crate) fn configure() {
    let exe_dir = std::env::current_exe().ok().and_then(|path| path.parent().map(Path::to_path_buf));
    let per_user_data_root =
        per_user_local_data_root(std::env::var_os("LOCALAPPDATA"), std::env::var_os("USERPROFILE"))
            .unwrap_or_else(std::env::temp_dir);
    let fixed_runtime = exe_dir.as_deref().and_then(|exe_dir| {
        fixed_runtime_paths_from_inputs(
            Some(exe_dir),
            exe_dir.join(PORTABLE_MARKER).is_file(),
            exe_dir.join(FIXED_WEBVIEW2_MARKER).is_file(),
            exe_dir.join(FIXED_WEBVIEW2_RUNTIME_DIR).join(FIXED_WEBVIEW2_EXECUTABLE).is_file(),
            exe_dir.join(MULTI_USER_MARKER).is_file(),
            Some(&per_user_data_root),
        )
    });
    let force_sandbox = matches!(std::env::var(FIXED_WEBVIEW2_FORCE_SANDBOX_ENV).as_deref(), Ok("1"));
    let request_no_sandbox = matches!(std::env::var(WEBVIEW2_NO_SANDBOX_ENV).as_deref(), Ok("1"));
    let environment = resolve_environment(
        fixed_runtime,
        std::env::var_os(WEBVIEW2_BROWSER_EXECUTABLE_FOLDER_ENV),
        std::env::var_os(WEBVIEW2_USER_DATA_FOLDER_ENV),
        std::env::var_os(WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS_ENV),
        force_sandbox,
        request_no_sandbox,
    );

    set_environment_variable(WEBVIEW2_BROWSER_EXECUTABLE_FOLDER_ENV, environment.browser_executable_folder);
    set_environment_variable(WEBVIEW2_USER_DATA_FOLDER_ENV, environment.user_data_folder);
    set_environment_variable(WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS_ENV, environment.additional_browser_arguments);
}

#[cfg(target_os = "windows")]
fn set_environment_variable(name: &str, value: Option<OsString>) {
    if let Some(value) = value {
        std::env::set_var(name, value);
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn configure() {}

#[cfg(test)]
mod tests {
    use super::{
        fixed_runtime_paths_from_inputs, per_user_local_data_root, resolve_environment, FixedRuntimePaths,
        FIXED_WEBVIEW2_RUNTIME_DIR,
    };
    use std::ffi::OsString;
    use std::path::PathBuf;

    fn fixed_runtime() -> FixedRuntimePaths {
        let exe_dir = PathBuf::from(r"D:\DBX");
        FixedRuntimePaths {
            browser_executable_folder: exe_dir.join(FIXED_WEBVIEW2_RUNTIME_DIR),
            user_data_folder: exe_dir.join("data").join("webview2"),
        }
    }

    #[test]
    fn detects_only_complete_fixed_runtime_packages() {
        let exe_dir = PathBuf::from(r"D:\DBX");
        assert_eq!(
            fixed_runtime_paths_from_inputs(Some(&exe_dir), true, true, true, false, None),
            Some(fixed_runtime())
        );
        assert!(fixed_runtime_paths_from_inputs(Some(&exe_dir), false, true, true, false, None).is_none());
        assert!(fixed_runtime_paths_from_inputs(Some(&exe_dir), true, false, true, false, None).is_none());
        assert!(fixed_runtime_paths_from_inputs(Some(&exe_dir), true, true, false, false, None).is_none());
        assert!(fixed_runtime_paths_from_inputs(None, true, true, true, false, None).is_none());
    }

    #[test]
    fn keeps_webview2_user_data_separate_for_each_windows_user() {
        let exe_dir = PathBuf::from(r"D:\SharedApps\DBX");
        let alice_local_data = PathBuf::from(r"C:\Users\alice\AppData\Local");
        let bob_local_data = PathBuf::from(r"C:\Users\bob\AppData\Local");

        let alice =
            fixed_runtime_paths_from_inputs(Some(&exe_dir), true, true, true, true, Some(&alice_local_data)).unwrap();
        let bob =
            fixed_runtime_paths_from_inputs(Some(&exe_dir), true, true, true, true, Some(&bob_local_data)).unwrap();

        assert_eq!(alice.user_data_folder, alice_local_data.join("com.dbx.app").join("WebView2").join("FixedRuntime"));
        assert_eq!(bob.user_data_folder, bob_local_data.join("com.dbx.app").join("WebView2").join("FixedRuntime"));
        assert_ne!(alice.user_data_folder, bob.user_data_folder);
    }

    #[test]
    fn resolves_windows_local_data_root_with_user_profile_fallback() {
        assert_eq!(
            per_user_local_data_root(
                Some(OsString::from(r"C:\Users\alice\AppData\Local")),
                Some(OsString::from(r"C:\Users\ignored")),
            ),
            Some(PathBuf::from(r"C:\Users\alice\AppData\Local"))
        );
        assert_eq!(
            per_user_local_data_root(None, Some(OsString::from(r"C:\Users\bob"))),
            Some(PathBuf::from(r"C:\Users\bob").join("AppData").join("Local"))
        );
    }

    #[test]
    fn configures_fixed_runtime_user_data_and_no_sandbox_defaults() {
        let environment = resolve_environment(Some(fixed_runtime()), None, None, None, false, false);

        assert_eq!(
            environment.browser_executable_folder,
            Some(PathBuf::from(r"D:\DBX").join("WebView2Runtime").into_os_string())
        );
        assert_eq!(
            environment.user_data_folder,
            Some(PathBuf::from(r"D:\DBX").join("data").join("webview2").into_os_string())
        );
        assert_eq!(environment.additional_browser_arguments, Some(OsString::from("--no-sandbox")));
    }

    #[test]
    fn explicit_webview2_paths_take_precedence_over_package_defaults() {
        let environment = resolve_environment(
            Some(fixed_runtime()),
            Some(OsString::from(r"E:\Managed\WebView2")),
            Some(OsString::from(r"E:\Managed\UserData")),
            None,
            false,
            false,
        );

        assert_eq!(environment.browser_executable_folder, Some(OsString::from(r"E:\Managed\WebView2")));
        assert_eq!(environment.user_data_folder, Some(OsString::from(r"E:\Managed\UserData")));
    }

    #[test]
    fn preserves_browser_arguments_and_does_not_duplicate_no_sandbox() {
        let environment = resolve_environment(
            Some(fixed_runtime()),
            None,
            None,
            Some(OsString::from("--disable-gpu --no-sandbox")),
            false,
            false,
        );
        assert_eq!(environment.additional_browser_arguments, Some(OsString::from("--disable-gpu --no-sandbox")));
    }

    #[test]
    fn force_sandbox_override_keeps_fixed_runtime_without_no_sandbox() {
        let environment =
            resolve_environment(Some(fixed_runtime()), None, None, Some(OsString::from("--disable-gpu")), true, true);
        assert_eq!(environment.additional_browser_arguments, Some(OsString::from("--disable-gpu")));
    }

    #[test]
    fn normal_builds_are_unchanged_without_explicit_compatibility_override() {
        let environment = resolve_environment(None, None, None, None, false, false);
        assert_eq!(
            environment,
            super::WebView2Environment {
                browser_executable_folder: None,
                user_data_folder: None,
                additional_browser_arguments: None,
            }
        );
    }

    #[test]
    fn legacy_no_sandbox_override_still_works_for_normal_builds() {
        let environment = resolve_environment(None, None, None, Some(OsString::from("--disable-gpu")), false, true);
        assert_eq!(environment.additional_browser_arguments, Some(OsString::from("--disable-gpu --no-sandbox")));
    }
}
