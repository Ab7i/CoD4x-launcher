use super::module;
use std::io::{Read, Seek, SeekFrom};
use winapi::um::winnt::IMAGE_FILE_LARGE_ADDRESS_AWARE;

pub fn is_pure() -> bool {
    let module: *const u8 = 0x401000 as *const u8;
    let module_text_end: *const u8 = 0x690429 as *const u8;
    let rdata: *const u8 = 0x691520 as *const u8;
    let rdata_end: *const u8 = 0x71b000 as *const u8;

    let text_len = unsafe { module_text_end.offset_from(module) as usize };
    let rdata_len = unsafe { rdata_end.offset_from(rdata) as usize };

    let text_checksum = adler32(module, text_len);
    if text_checksum != 0xD0D368F6 {
        return false;
    }

    let rdata_checksum = adler32(rdata, rdata_len);
    if rdata_checksum != 0xAA33BC12 {
        return false;
    }

    true
}

pub fn is_large_address_aware() -> bool {
    is_large_address_aware_impl().unwrap_or(true)
}

pub fn startup() -> i32 {
    unsafe {
        __iw3mp_security_init_cookie();
        __iw3mp_tmainCRTStartup()
    }
}

fn is_large_address_aware_impl() -> std::io::Result<bool> {
    let mut file = std::fs::File::open(module::get_path())?;
    file.seek(SeekFrom::Start(286))?;

    let mut buffer = [0u8; 2];
    file.read_exact(&mut buffer)?;

    let flags = u16::from_le_bytes(buffer);
    Ok(flags & IMAGE_FILE_LARGE_ADDRESS_AWARE != 0)
}

fn adler32(data: *const u8, len: usize) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;

    const PRIME: u32 = 65521;
    for i in 0..len {
        a = unsafe { (a + *data.add(i) as u32) % PRIME };
        b = (b + a) % PRIME;
    }

    (b << 16) | a
}

unsafe fn __iw3mp_security_init_cookie() {
    type CdeclFn = unsafe extern "C" fn();
    let func: CdeclFn = std::mem::transmute(0x67f189_usize);
    func()
}

#[allow(non_snake_case)]
unsafe fn __iw3mp_tmainCRTStartup() -> i32 {
    type CdeclFn = unsafe extern "C" fn() -> i32;
    let func: CdeclFn = std::mem::transmute(0x67475c_usize);
    func()
}
