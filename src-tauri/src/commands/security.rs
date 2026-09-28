use std::sync::Mutex;
use tauri::State;
use zeroize::Zeroizing;

use crate::state::AppState;
use crate::vault::keystore::KeyStore;
use crate::vault::lifecycle;

/// The real Keychain (macOS). Every Keychain call in the app goes through
/// this type via the `KeyStore` trait so tests can substitute a fake (C6).
pub struct SystemKeyStore;

impl KeyStore for SystemKeyStore {
    fn store(&self, dek: &[u8; 32]) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        { macos::store_key(dek) }
        #[cfg(not(target_os = "macos"))]
        { let _ = dek; Err("Touch ID not supported on this platform".into()) }
    }

    fn read(&self) -> Result<Zeroizing<Vec<u8>>, String> {
        #[cfg(target_os = "macos")]
        { macos::read_key() }
        #[cfg(not(target_os = "macos"))]
        { Err("Touch ID not supported".into()) }
    }

    fn delete(&self) -> Result<(), String> {
        #[cfg(target_os = "macos")]
        { macos::delete_key() }
        #[cfg(not(target_os = "macos"))]
        { Ok(()) }
    }
}

#[tauri::command]
pub fn is_touch_id_available() -> bool {
    #[cfg(target_os = "macos")]
    { macos::check_biometry() }
    #[cfg(not(target_os = "macos"))]
    { false }
}

/// Store a copy of the DEK (never a password KEK) in the Keychain.
#[tauri::command]
pub fn setup_touch_id(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let app = state.lock().map_err(|_| "Lock poisoned".to_string())?;
    let dek = app.dek.as_ref().ok_or("Vault is locked")?;
    lifecycle::enable_touch_id(&app.data_dir, dek, app.legacy, &SystemKeyStore)
}

#[tauri::command]
pub fn disable_touch_id() -> Result<(), String> {
    SystemKeyStore.delete()
}

#[tauri::command]
pub fn unlock_biometric(state: State<'_, Mutex<AppState>>) -> Result<(), String> {
    let mut app = state.lock().map_err(|_| "Lock poisoned".to_string())?;
    if app.is_unlocked() {
        return Ok(());
    }
    let unlocked = lifecycle::unlock_with_keystore(&app.data_dir, &SystemKeyStore)?;
    app.set_unlocked(unlocked);
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos {
    use std::ffi::c_void;
    use std::ptr;

    use core_foundation::base::{CFTypeRef, TCFType, kCFAllocatorDefault};
    use core_foundation::boolean::CFBoolean;
    use core_foundation::data::CFData;
    use core_foundation::string::CFString;

    use security_framework_sys::base::errSecSuccess;

    // Link frameworks
    #[link(name = "LocalAuthentication", kind = "framework")]
    extern "C" {}

    #[link(name = "Security", kind = "framework")]
    extern "C" {
        static kSecClass: CFTypeRef;
        static kSecClassGenericPassword: CFTypeRef;
        static kSecAttrService: CFTypeRef;
        static kSecAttrAccount: CFTypeRef;
        static kSecValueData: CFTypeRef;
        static kSecReturnData: CFTypeRef;
        static kSecAttrAccessControl: CFTypeRef;
        static kSecMatchLimit: CFTypeRef;
        static kSecMatchLimitOne: CFTypeRef;
        static kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly: CFTypeRef;

        fn SecAccessControlCreateWithFlags(
            allocator: CFTypeRef,
            protection: CFTypeRef,
            flags: usize,
            error: *mut CFTypeRef,
        ) -> CFTypeRef;

        fn SecItemAdd(
            attributes: CFTypeRef,
            result: *mut CFTypeRef,
        ) -> i32;

        fn SecItemCopyMatching(
            query: CFTypeRef,
            result: *mut CFTypeRef,
        ) -> i32;

        fn SecItemDelete(query: CFTypeRef) -> i32;

        fn CFDictionaryCreateMutable(
            allocator: CFTypeRef,
            capacity: isize,
            key_callbacks: *const c_void,
            value_callbacks: *const c_void,
        ) -> CFTypeRef;

        fn CFDictionarySetValue(
            dict: CFTypeRef,
            key: CFTypeRef,
            value: CFTypeRef,
        );

        static kCFTypeDictionaryKeyCallBacks: c_void;
        static kCFTypeDictionaryValueCallBacks: c_void;

        fn CFRelease(cf: CFTypeRef);
    }

    const SERVICE: &str = "com.vaultx.app";
    // Historical account name; the item now holds the DEK (contract C6).
    const ACCOUNT: &str = "master-key";
    // kSecAccessControlBiometryCurrentSet = 1 << 3
    const BIOMETRY_CURRENT_SET: usize = 8;

    /// Check if Touch ID hardware is available and enrolled via LAContext
    pub fn check_biometry() -> bool {
        unsafe {
            let cls = objc2::runtime::AnyClass::get("LAContext");
            let Some(cls) = cls else { return false };

            let obj: objc2::rc::Retained<objc2::runtime::AnyObject> =
                objc2::msg_send_id![cls, new];

            let mut error: *mut objc2::runtime::AnyObject = ptr::null_mut();
            // LAPolicyDeviceOwnerAuthenticationWithBiometrics = 1
            let available: bool =
                objc2::msg_send![&*obj, canEvaluatePolicy: 1_isize, error: &mut error];
            available
        }
    }

    /// Store the DEK in Keychain with biometric access control
    pub fn store_key(key: &[u8]) -> Result<(), String> {
        unsafe {
            let service = CFString::new(SERVICE);
            let account = CFString::new(ACCOUNT);
            let data = CFData::from_buffer(key);

            // Create access control requiring biometry (current set)
            let mut error: CFTypeRef = ptr::null_mut();
            let access_control = SecAccessControlCreateWithFlags(
                kCFAllocatorDefault as _,
                kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly,
                BIOMETRY_CURRENT_SET,
                &mut error,
            );
            if access_control.is_null() {
                return Err("Failed to create biometric access control".into());
            }

            let dict = CFDictionaryCreateMutable(
                kCFAllocatorDefault as _,
                5,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            );
            CFDictionarySetValue(dict, kSecClass, kSecClassGenericPassword);
            CFDictionarySetValue(dict, kSecAttrService, service.as_concrete_TypeRef() as CFTypeRef);
            CFDictionarySetValue(dict, kSecAttrAccount, account.as_concrete_TypeRef() as CFTypeRef);
            CFDictionarySetValue(dict, kSecValueData, data.as_concrete_TypeRef() as CFTypeRef);
            CFDictionarySetValue(dict, kSecAttrAccessControl, access_control);

            let status = SecItemAdd(dict, ptr::null_mut());

            CFRelease(dict);
            CFRelease(access_control);

            if status == errSecSuccess {
                Ok(())
            } else {
                Err(format!("Keychain store failed (OSStatus {status})"))
            }
        }
    }

    /// Read the DEK from Keychain — triggers Touch ID prompt
    pub fn read_key() -> Result<zeroize::Zeroizing<Vec<u8>>, String> {
        unsafe {
            let service = CFString::new(SERVICE);
            let account = CFString::new(ACCOUNT);

            let dict = CFDictionaryCreateMutable(
                kCFAllocatorDefault as _,
                5,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            );
            CFDictionarySetValue(dict, kSecClass, kSecClassGenericPassword);
            CFDictionarySetValue(dict, kSecAttrService, service.as_concrete_TypeRef() as CFTypeRef);
            CFDictionarySetValue(dict, kSecAttrAccount, account.as_concrete_TypeRef() as CFTypeRef);
            CFDictionarySetValue(
                dict,
                kSecReturnData,
                CFBoolean::true_value().as_concrete_TypeRef() as CFTypeRef,
            );
            CFDictionarySetValue(dict, kSecMatchLimit, kSecMatchLimitOne);

            let mut result: CFTypeRef = ptr::null_mut();
            let status = SecItemCopyMatching(dict, &mut result);

            CFRelease(dict);

            if status != errSecSuccess || result.is_null() {
                return Err("Touch ID authentication failed or key not found".into());
            }

            // result is a retained CFDataRef
            let cf_data = CFData::wrap_under_create_rule(result as _);
            Ok(zeroize::Zeroizing::new(cf_data.bytes().to_vec()))
        }
    }

    /// Remove the DEK from Keychain
    pub fn delete_key() -> Result<(), String> {
        unsafe {
            let service = CFString::new(SERVICE);
            let account = CFString::new(ACCOUNT);

            let dict = CFDictionaryCreateMutable(
                kCFAllocatorDefault as _,
                3,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            );
            CFDictionarySetValue(dict, kSecClass, kSecClassGenericPassword);
            CFDictionarySetValue(dict, kSecAttrService, service.as_concrete_TypeRef() as CFTypeRef);
            CFDictionarySetValue(dict, kSecAttrAccount, account.as_concrete_TypeRef() as CFTypeRef);

            let status = SecItemDelete(dict);

            CFRelease(dict);

            // errSecItemNotFound (-25300) is OK for delete
            if status == errSecSuccess || status == -25300 {
                Ok(())
            } else {
                Err(format!("Keychain delete failed (OSStatus {status})"))
            }
        }
    }
}
