use winapi::shared::basetsd::SIZE_T;
use winapi::shared::minwindef::{DWORD, HINSTANCE, LPVOID};
use winapi::um::processthreadsapi::{CreateThread, ExitThread};

#[cfg(feature = "cod4v17_patch")]
use cod4v17_patch::patch;

use core::ffi::{c_char, c_void};

use super::cod4x;
use super::filesystem as fs;
use super::iw3mp;
use super::miles32;
use super::module;
use super::msg_box::*;

extern "system" fn run(hinstance: LPVOID) -> DWORD {
    let hinstance = hinstance as HINSTANCE;
    let cmdline_args: Vec<_> = std::env::args().collect();
    let legacy_arg = get_cmdline_value("legacymode", &cmdline_args);
    let run_legacy = legacy_arg.is_some_and(|v| v == "1");
    if !run_legacy {
        let version = get_cmdline_value("protocolversion", &cmdline_args);
        match cod4x::run(hinstance, version) {
            Err(e) => {
                message_box(format!("{e}").as_str(), "Error");
            }
            Ok(_) => unreachable!(),
        };
    }

    #[cfg(feature = "cod4v17_patch")]
    if let Err(e) = patch::patch_iw3mp() {
        message_box(
            format!("Failed to patch iw3mp.exe: {e}\nContinue at your own risk!").as_str(),
            "Error",
        );
    }

    iw3mp::startup() as DWORD
}

fn run_thread(hinstance: HINSTANCE) {
    unsafe {
        let thread_handle = CreateThread(
            std::ptr::null_mut(),
            1024 * 1024 * 12 as SIZE_T,
            Some(run),
            hinstance as LPVOID,
            0 as DWORD,
            std::ptr::null_mut(),
        );

        if !thread_handle.is_null() {
            ExitThread(0);
        }
    }
}

#[no_mangle]
#[allow(non_snake_case)]
extern "C" fn StartLauncher(
    hinstance: HINSTANCE,
    mss32importprocs: *mut *mut c_void,
    mss32importnames: *const *const c_char,
    mss32importcount: i32,
) {
    let module_path = module::get_path();
    if let Some(install_dir) = module_path.parent() {
        fs::set_current_directory(install_dir);
    }

    // Sandbox build: no auto-fix, no network. A clean iw3mp.exe v1.7 is
    // required up front -- bail out cleanly instead of crashing on the
    // hardcoded WinMain addresses if it is not.
    if !iw3mp::is_pure() || !iw3mp::is_large_address_aware() {
        message_box(
            "Impure or non-large-address-aware iw3mp.exe detected.\n\
             A clean iw3mp.exe v1.7 is required to continue.",
            "CoD4x Launcher",
        );
        std::process::exit(0);
    }

    // Load miles32.dll locally and forward the Miles import table that the
    // mss32 proxy handed us. No replacement / download on failure.
    let _miles32 = match miles32::load_module(mss32importprocs, mss32importnames, mss32importcount)
    {
        Ok(lib) => lib,
        Err(e) => {
            message_box(
                format!("Failed to load miles32.dll: {e}").as_str(),
                "CoD4x Launcher",
            );
            return;
        }
    };

    run_thread(hinstance);
}

fn get_cmdline_value<'a>(value_name: &str, cmdline_args: &'a [String]) -> Option<&'a String> {
    let mut args = cmdline_args.iter();
    while let Some(arg) = args.next() {
        if arg == "+set" || arg == "+seta" {
            if let Some(arg) = args.next() {
                if arg == value_name {
                    return args.next();
                }
            }
        }
    }
    None
}
