use super::module;
use core::ffi::{c_char, c_void, CStr};
use libloading::Library;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

pub fn load_module(
    mss32importprocs: *mut *mut c_void,
    mss32importnames: *const *const c_char,
    mss32importcount: i32,
) -> Result<libloading::Library, Miles32LoadError> {
    let names = c_strings_to_slices(mss32importnames, mss32importcount);

    let miles32path = std::path::Path::new("miles32.dll");
    let module_path = module::get_path();
    let install_dir = module_path.parent();

    let full_miles32path = if let Some(install_dir) = install_dir {
        install_dir.join(miles32path)
    } else {
        std::path::PathBuf::from(miles32path)
    };

    unsafe {
        let Ok(lib) = Library::new(full_miles32path) else {
            return Err(Miles32LoadError::ModuleNotFound);
        };

        for (i, name) in names.iter().enumerate() {
            *mss32importprocs.add(i) = lib
                .get::<*mut core::ffi::c_void>(*name)
                .ok()
                .and_then(|p| p.try_as_raw_ptr())
                .ok_or_else(|| {
                    Miles32LoadError::MissingProcedure(
                        convert_bytes_to_string(name).unwrap_or("<Error>".to_string()),
                    )
                })?;
        }

        Ok(lib)
    }
}

fn c_strings_to_slices<'a>(ptr: *const *const c_char, count: i32) -> Vec<&'a [u8]> {
    let mut slices = Vec::new();

    unsafe {
        for i in 0..count {
            let c_str_ptr = *ptr.add(i as usize);
            if !c_str_ptr.is_null() {
                let c_str = CStr::from_ptr(c_str_ptr);
                slices.push(c_str.to_bytes());
            }
        }
    }

    slices
}

fn convert_bytes_to_string(bytes: &[u8]) -> Result<String, std::str::Utf8Error> {
    let string_slice = std::str::from_utf8(bytes)?;
    Ok(string_slice.to_string())
}

pub enum Miles32LoadError {
    ModuleNotFound,
    MissingProcedure(String),
}

impl Miles32LoadError {
    fn message(&self) -> String {
        match self {
            Self::ModuleNotFound => "Miles32 DLL not found".to_string(),
            Self::MissingProcedure(name) => format!("Missing Miles32 procedure '{name}'"),
        }
    }
}

impl Display for Miles32LoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.message())
    }
}

impl Debug for Miles32LoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.message())
    }
}

impl Error for Miles32LoadError {}
