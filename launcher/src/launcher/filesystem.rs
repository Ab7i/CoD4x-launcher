use super::wstring;
use winapi::um::processenv::SetCurrentDirectoryW;
use winapi::um::winbase::SetDllDirectoryW;

pub fn get_appdata_cod4_path() -> anyhow::Result<std::path::PathBuf> {
    use std::path::PathBuf;

    // Priority 1: the LOCALAPPDATA environment variable.
    // The mss32 proxy sets this to the sandbox fake-appdata before the
    // launcher runs. Unlike SHGetKnownFolderPath, the env var honours that
    // override, so it must be consulted first.
    // Priority 2 (fallback): SHGetKnownFolderPath(FOLDERID_LocalAppData),
    // used only when LOCALAPPDATA is absent from the environment.
    let app_data = match std::env::var("LOCALAPPDATA") {
        Ok(value) => value,
        Err(env_err) => get_known_local_appdata().ok_or(env_err)?,
    };

    Ok(PathBuf::from(app_data).join("CallofDuty4MW"))
}

fn get_known_local_appdata() -> Option<String> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use winapi::shared::minwindef::LPVOID;
    use winapi::shared::ntdef::LPWSTR;
    use winapi::shared::winerror::SUCCEEDED;
    use winapi::um::combaseapi::CoTaskMemFree;
    use winapi::um::knownfolders::FOLDERID_LocalAppData;
    use winapi::um::shlobj::{SHGetKnownFolderPath, KF_FLAG_CREATE};

    unsafe {
        let mut res: LPWSTR = std::ptr::null_mut();
        let status = SHGetKnownFolderPath(
            &FOLDERID_LocalAppData,
            KF_FLAG_CREATE,
            std::ptr::null_mut(),
            &mut res,
        );

        if SUCCEEDED(status) && !res.is_null() {
            let len = (0..).take_while(|&i| *res.add(i) != 0).count();
            let wide_slice = std::slice::from_raw_parts(res, len);
            let path = OsString::from_wide(wide_slice).into_string();
            CoTaskMemFree(res as LPVOID);
            path.ok()
        } else {
            None
        }
    }
}

pub fn appdata_bin_path() -> anyhow::Result<std::path::PathBuf> {
    Ok(get_appdata_cod4_path()?.join("bin"))
}

pub fn set_current_directory(path: &std::path::Path) {
    unsafe {
        SetCurrentDirectoryW(wstring::Wstring::new(path).into());
    }
}

pub fn set_dll_directory(path: &std::path::Path) {
    unsafe {
        SetDllDirectoryW(wstring::Wstring::new(path).into());
    }
}
