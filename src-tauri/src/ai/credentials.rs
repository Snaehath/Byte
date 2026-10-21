const CLOUD_API_KEY_TARGET: &str = "Byte:CloudModelApiKey";

#[cfg(windows)]
mod windows_vault {
    use std::ffi::c_void;

    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[repr(C)]
    struct CredentialW {
        flags: u32,
        kind: u32,
        target_name: *mut u16,
        comment: *mut u16,
        last_written: FileTime,
        credential_blob_size: u32,
        credential_blob: *mut u8,
        persist: u32,
        attribute_count: u32,
        attributes: *mut c_void,
        target_alias: *mut u16,
        user_name: *mut u16,
    }

    #[link(name = "Advapi32")]
    extern "system" {
        fn CredWriteW(credential: *const CredentialW, flags: u32) -> i32;
        fn CredReadW(target: *const u16, kind: u32, flags: u32, credential: *mut *mut CredentialW) -> i32;
        fn CredFree(buffer: *mut c_void);
    }

    const CRED_TYPE_GENERIC: u32 = 1;
    const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;

    pub fn store(target: &str, secret: &str) -> Result<(), String> {
        let mut target: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        let mut blob = secret.as_bytes().to_vec();
        let credential = CredentialW {
            flags: 0,
            kind: CRED_TYPE_GENERIC,
            target_name: target.as_mut_ptr(),
            comment: std::ptr::null_mut(),
            last_written: FileTime { low: 0, high: 0 },
            credential_blob_size: blob.len() as u32,
            credential_blob: blob.as_mut_ptr(),
            persist: CRED_PERSIST_LOCAL_MACHINE,
            attribute_count: 0,
            attributes: std::ptr::null_mut(),
            target_alias: std::ptr::null_mut(),
            user_name: std::ptr::null_mut(),
        };

        let result = unsafe { CredWriteW(&credential, 0) };
        if result == 0 {
            Err(format!("Windows Credential Manager could not save the cloud API key (error {}).", std::io::Error::last_os_error()))
        } else {
            Ok(())
        }
    }

    pub fn read(target: &str) -> Result<Option<String>, String> {
        let target: Vec<u16> = target.encode_utf16().chain(std::iter::once(0)).collect();
        let mut credential = std::ptr::null_mut();
        let result = unsafe { CredReadW(target.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
        if result == 0 {
            return Ok(None);
        }

        let bytes = unsafe {
            let entry = &*credential;
            if entry.credential_blob.is_null() || entry.credential_blob_size == 0 {
                None
            } else {
                let bytes = std::slice::from_raw_parts(entry.credential_blob, entry.credential_blob_size as usize);
                Some(bytes.to_vec())
            }
        };
        unsafe { CredFree(credential.cast()) };
        bytes
            .map(|bytes| String::from_utf8(bytes).map_err(|_| "Saved cloud API key is not valid UTF-8.".to_string()))
            .transpose()
    }
}

pub fn store_cloud_api_key(secret: &str) -> Result<(), String> {
    #[cfg(windows)]
    { windows_vault::store(CLOUD_API_KEY_TARGET, secret) }
    #[cfg(not(windows))]
    { let _ = secret; Err("Cloud API key storage is currently supported on Windows only.".to_string()) }
}

pub fn cloud_api_key() -> Result<Option<String>, String> {
    #[cfg(windows)]
    { windows_vault::read(CLOUD_API_KEY_TARGET) }
    #[cfg(not(windows))]
    { Ok(None) }
}

