//! iOS Mobile Installation Agent (misagent) Client Bindings
//!
//! Provides C-compatible bindings for interacting with the misagent service on iOS devices.

use std::ptr::null_mut;

use idevice::{IdeviceError, IdeviceService, misagent::MisagentClient, provider::IdeviceProvider};

use crate::{IdeviceFfiError, ffi_err, provider::IdeviceProviderHandle, run_sync_local};
#[cfg(all(feature = "core_device_proxy", feature = "rsd"))]
use crate::{core_device_proxy::AdapterHandle, rsd::RsdHandshakeHandle};
#[cfg(all(feature = "core_device_proxy", feature = "rsd"))]
use idevice::RsdService as _;

pub struct MisagentClientHandle(pub MisagentClient);

/// Automatically creates and connects to Misagent, returning a client handle
///
/// # Arguments
/// * [`provider`] - An IdeviceProvider
/// * [`client`] - On success, will be set to point to a newly allocated MisagentClient handle
///
/// # Returns
/// An IdeviceFfiError on error, null on success
///
/// # Safety
/// `provider` must be a valid pointer to a handle allocated by this library
/// `client` must be a valid, non-null pointer to a location where the handle will be stored
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_connect(
    provider: *mut IdeviceProviderHandle,
    client: *mut *mut MisagentClientHandle,
) -> *mut IdeviceFfiError {
    if provider.is_null() || client.is_null() {
        tracing::error!("Null pointer provided");
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }

    let res: Result<MisagentClient, IdeviceError> = run_sync_local(async move {
        let provider_ref: &dyn IdeviceProvider = unsafe { &*(*provider).0 };
        MisagentClient::connect(provider_ref).await
    });

    match res {
        Ok(r) => {
            let boxed = Box::new(MisagentClientHandle(r));
            unsafe { *client = Box::into_raw(boxed) };
            null_mut()
        }
        Err(e) => ffi_err!(e),
    }
}

/// Creates a new MisagentClient via RSD
///
/// # Arguments
/// * [`provider`] - An adapter created by this library
/// * [`handshake`] - An RSD handshake from the same provider
/// * [`client`] - On success, will be set to point to a newly allocated MisagentClient handle
///
/// # Returns
/// An IdeviceFfiError on error, null on success
///
/// # Safety
/// `provider` must be a valid pointer to a handle allocated by this library
/// `handshake` must be a valid pointer to a handle allocated by this library
/// `client` must be a valid, non-null pointer to a location where the handle will be stored
#[cfg(all(feature = "core_device_proxy", feature = "rsd"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_connect_rsd(
    provider: *mut AdapterHandle,
    handshake: *mut RsdHandshakeHandle,
    client: *mut *mut MisagentClientHandle,
) -> *mut IdeviceFfiError {
    if provider.is_null() || handshake.is_null() || client.is_null() {
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }
    let res: Result<MisagentClient, IdeviceError> = run_sync_local(async move {
        let provider_ref = unsafe { &mut (*provider).0 };
        let handshake_ref = unsafe { &mut (*handshake).0 };
        MisagentClient::connect_rsd(provider_ref, handshake_ref).await
    });

    match res {
        Ok(r) => {
            let boxed = Box::new(MisagentClientHandle(r));
            unsafe { *client = Box::into_raw(boxed) };
            null_mut()
        }
        Err(e) => ffi_err!(e),
    }
}

/// Installs a provisioning profile on the device
///
/// # Arguments
/// * [`client`] - A valid MisagentClient handle
/// * [`profile_data`] - The provisioning profile data to install
/// * [`profile_len`] - Length of the profile data
///
/// # Returns
/// An IdeviceFfiError on error, null on success
///
/// # Safety
/// `client` must be a valid pointer to a handle allocated by this library
/// `profile_data` must be a valid pointer to profile data of length `profile_len`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_install(
    client: *mut MisagentClientHandle,
    profile_data: *const u8,
    profile_len: libc::size_t,
) -> *mut IdeviceFfiError {
    if client.is_null() || profile_data.is_null() {
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }

    let profile = unsafe { std::slice::from_raw_parts(profile_data, profile_len) }.to_vec();

    let res = run_sync_local(async { unsafe { &mut *client }.0.install(profile).await });

    match res {
        Ok(_) => null_mut(),
        Err(e) => ffi_err!(e),
    }
}

/// Removes a provisioning profile from the device
///
/// # Arguments
/// * [`client`] - A valid MisagentClient handle
/// * [`profile_id`] - The UUID of the profile to remove (C string)
///
/// # Returns
/// An IdeviceFfiError on error, null on success
///
/// # Safety
/// `client` must be a valid pointer to a handle allocated by this library
/// `profile_id` must be a valid C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_remove(
    client: *mut MisagentClientHandle,
    profile_id: *const libc::c_char,
) -> *mut IdeviceFfiError {
    if client.is_null() || profile_id.is_null() {
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }

    let id = unsafe { std::ffi::CStr::from_ptr(profile_id) }
        .to_string_lossy()
        .into_owned();

    let res = run_sync_local(async { unsafe { &mut *client }.0.remove(&id).await });

    match res {
        Ok(_) => null_mut(),
        Err(e) => ffi_err!(e),
    }
}

/// Retrieves all provisioning profiles from the device
///
/// # Arguments
/// * [`client`] - A valid MisagentClient handle
/// * [`out_profiles`] - On success, will be set to point to an array of profile data
/// * [`out_profiles_len`] - On success, will be set to the number of profiles
///
/// # Returns
/// An IdeviceFfiError on error, null on success
///
/// # Safety
/// `client` must be a valid pointer to a handle allocated by this library
/// `out_profiles` must be a valid pointer to store the resulting array
/// `out_profiles_len` must be a valid pointer to store the array length
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_copy_all(
    client: *mut MisagentClientHandle,
    out_profiles: *mut *mut *mut u8,
    out_profiles_len: *mut *mut libc::size_t,
    out_count: *mut libc::size_t,
) -> *mut IdeviceFfiError {
    if client.is_null()
        || out_profiles.is_null()
        || out_profiles_len.is_null()
        || out_count.is_null()
    {
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }

    let res: Result<Vec<Vec<u8>>, IdeviceError> =
        run_sync_local(async { unsafe { &mut *client }.0.copy_all().await });

    unsafe { export_profiles(res, out_profiles, out_profiles_len, out_count) }
}

// No FFI pointers escape before the read-only transaction fully completes.
unsafe fn export_profiles(
    res: Result<Vec<Vec<u8>>, IdeviceError>,
    out_profiles: *mut *mut *mut u8, out_profiles_len: *mut *mut libc::size_t,
    out_count: *mut libc::size_t,
) -> *mut IdeviceFfiError {
    match res {
        Ok(profiles) => {
            let count = profiles.len();
            let mut profile_ptrs = Vec::with_capacity(count);
            let mut profile_lens = Vec::with_capacity(count);

            for profile in profiles {
                let len = profile.len();
                let mut boxed_profile = profile.into_boxed_slice();
                let ptr = boxed_profile.as_mut_ptr();
                std::mem::forget(boxed_profile);
                profile_ptrs.push(ptr);
                profile_lens.push(len);
            }

            let boxed_ptrs = profile_ptrs.into_boxed_slice();
            let boxed_lens = profile_lens.into_boxed_slice();

            unsafe {
                *out_profiles = Box::into_raw(boxed_ptrs) as *mut *mut u8;
                *out_profiles_len = Box::into_raw(boxed_lens) as *mut libc::size_t;
                *out_count = count;
            }

            null_mut()
        }
        Err(e) => ffi_err!(e),
    }
}

const PROFILE_READBACK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

async fn bounded_profile_readback<F>(query: F, limit: std::time::Duration) -> Result<Vec<Vec<u8>>, IdeviceError>
where F: std::future::Future<Output = Result<Vec<Vec<u8>>, IdeviceError>> {
    tokio::time::timeout(limit, query).await.map_err(|_| IdeviceError::Timeout)?
}

/// Read installed profiles with a ten-second native deadline covering service
/// connection and CopyAll. The provider is borrowed; only the new query client
/// is dropped on completion or timeout. No installation or reconnect is issued.
/// # Safety
/// Provider must be a live exclusively-used handle for this call. Output pointers
/// must be writable. Returned arrays must be freed with misagent_free_profiles.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_read_profiles(
    provider: *mut IdeviceProviderHandle,
    out_profiles: *mut *mut *mut u8, out_profiles_len: *mut *mut libc::size_t,
    out_count: *mut libc::size_t,
) -> *mut IdeviceFfiError {
    if out_profiles.is_null() || out_profiles_len.is_null() || out_count.is_null() {
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }
    unsafe { *out_profiles = null_mut(); *out_profiles_len = null_mut(); *out_count = 0; }
    if provider.is_null() { return ffi_err!(IdeviceError::FfiInvalidArg); }
    let result = run_sync_local(bounded_profile_readback(async {
        let provider_ref: &dyn IdeviceProvider = unsafe { &*(*provider).0 };
        let mut client = MisagentClient::connect(provider_ref).await?;
        client.copy_all().await
    }, PROFILE_READBACK_TIMEOUT));
    unsafe { export_profiles(result, out_profiles, out_profiles_len, out_count) }
}

/// Read profiles over an existing RSD adapter/handshake with the same bounded
/// owned-client lifetime as misagent_read_profiles. No tunnel is created here.
/// # Safety
/// Adapter and handshake must be live, paired and exclusively borrowed for this
/// call. Output pointers must be writable; free results with misagent_free_profiles.
#[cfg(all(feature = "core_device_proxy", feature = "rsd"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_read_profiles_rsd(
    provider: *mut AdapterHandle, handshake: *mut RsdHandshakeHandle,
    out_profiles: *mut *mut *mut u8, out_profiles_len: *mut *mut libc::size_t,
    out_count: *mut libc::size_t,
) -> *mut IdeviceFfiError {
    if out_profiles.is_null() || out_profiles_len.is_null() || out_count.is_null() {
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }
    unsafe { *out_profiles = null_mut(); *out_profiles_len = null_mut(); *out_count = 0; }
    if provider.is_null() || handshake.is_null() { return ffi_err!(IdeviceError::FfiInvalidArg); }
    let result = run_sync_local(bounded_profile_readback(async {
        let provider_ref = unsafe { &mut (*provider).0 };
        let handshake_ref = unsafe { &mut (*handshake).0 };
        let mut client = MisagentClient::connect_rsd(provider_ref, handshake_ref).await?;
        client.copy_all().await
    }, PROFILE_READBACK_TIMEOUT));
    unsafe { export_profiles(result, out_profiles, out_profiles_len, out_count) }
}

/// Frees profiles array returned by misagent_copy_all
///
/// # Arguments
/// * [`profiles`] - Array of profile data pointers
/// * [`lens`] - Array of profile lengths
/// * [`count`] - Number of profiles in the array
///
/// # Safety
/// Must only be called with values returned from misagent_copy_all
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_free_profiles(
    profiles: *mut *mut u8,
    lens: *mut libc::size_t,
    count: libc::size_t,
) {
    if profiles.is_null() || lens.is_null() || count == 0 {
        return;
    }

    let profiles = unsafe { std::slice::from_raw_parts_mut(profiles, count) };
    let lens = unsafe { std::slice::from_raw_parts_mut(lens, count) };

    for (ptr, len) in profiles.iter_mut().zip(lens.iter()) {
        if !ptr.is_null() {
            let _ = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(*ptr, *len)) };
        }
    }

    let _ = unsafe { Box::from_raw(profiles as *mut [_]) };
    let _ = unsafe { Box::from_raw(lens as *mut [_]) };
}

/// Frees a misagent client handle
///
/// # Arguments
/// * [`handle`] - The handle to free
///
/// # Safety
/// `handle` must be a valid pointer to a handle allocated by this library,
/// or NULL (in which case this function does nothing)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn misagent_client_free(handle: *mut MisagentClientHandle) {
    if !handle.is_null() {
        tracing::debug!("Freeing misagent_client");
        let _ = unsafe { Box::from_raw(handle) };
    }
}

#[cfg(test)]
mod bounded_profile_readback_tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    struct QueryOwner(Arc<AtomicUsize>);
    impl Drop for QueryOwner { fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); } }

    #[derive(Debug)]
    struct FixtureProvider {
        drops: Arc<AtomicUsize>, query_drops: Arc<AtomicUsize>, pending: bool,
    }
    impl Drop for FixtureProvider {
        fn drop(&mut self) { self.drops.fetch_add(1, Ordering::SeqCst); }
    }
    impl IdeviceProvider for FixtureProvider {
        fn label(&self) -> &str { "non-secret-readback-fixture" }
        fn connect(&self, _port: u16) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<idevice::Idevice, IdeviceError>> + Send>> {
            let owner = QueryOwner(self.query_drops.clone());
            let pending = self.pending;
            Box::pin(async move {
                let _owner = owner;
                if pending { std::future::pending().await }
                else { Err(IdeviceError::UnexpectedResponse("fixture_connect_failure".into())) }
            })
        }
        fn get_pairing_file(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<idevice::pairing_file::PairingFile, IdeviceError>> + Send>> {
            Box::pin(async { Err(IdeviceError::UnexpectedResponse("fixture_pairing_not_requested".into())) })
        }
    }
    fn exercise_exported_transaction(pending: bool) {
        let drops = Arc::new(AtomicUsize::new(0));
        let query_drops = Arc::new(AtomicUsize::new(0));
        let mut provider = Box::new(IdeviceProviderHandle(Box::new(FixtureProvider {
            drops: drops.clone(), query_drops: query_drops.clone(), pending,
        })));
        let mut profiles = std::ptr::dangling_mut::<*mut u8>();
        let mut lengths = std::ptr::dangling_mut::<usize>();
        let mut count = 99;
        let error = unsafe { misagent_read_profiles(&mut *provider, &mut profiles, &mut lengths, &mut count) };
        assert!(!error.is_null());
        assert!(profiles.is_null() && lengths.is_null() && count == 0);
        assert_eq!(query_drops.load(Ordering::SeqCst), 1);
        assert_eq!(drops.load(Ordering::SeqCst), 0, "borrowed provider must survive");
        unsafe { crate::idevice_error_free(error) };
        drop(provider);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn exported_failure_keeps_provider_and_clears_outputs() { exercise_exported_transaction(false); }
    #[test]
    fn exported_deadline_drops_query_before_return_not_provider() { exercise_exported_transaction(true); }

    #[tokio::test]
    async fn timeout_drops_owned_query_before_return() {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = QueryOwner(drops.clone());
        let result = bounded_profile_readback(async move {
            let _owner = owner;
            std::future::pending::<Result<Vec<Vec<u8>>, IdeviceError>>().await
        }, std::time::Duration::from_millis(10)).await;
        assert!(matches!(result, Err(IdeviceError::Timeout)));
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }
    #[tokio::test]
    async fn successful_query_drops_owner_and_preserves_bytes() {
        let drops = Arc::new(AtomicUsize::new(0));
        let owner = QueryOwner(drops.clone());
        let result = bounded_profile_readback(async move {
            let _owner = owner; Ok(vec![vec![1,2,3]])
        }, std::time::Duration::from_secs(1)).await.unwrap();
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(result, vec![vec![1,2,3]]);
    }
    #[test]
    fn invalid_provider_zeros_outputs() {
        let mut profiles = std::ptr::dangling_mut::<*mut u8>();
        let mut lengths = std::ptr::dangling_mut::<usize>();
        let mut count = 999;
        let error = unsafe { misagent_read_profiles(null_mut(), &mut profiles, &mut lengths, &mut count) };
        assert!(!error.is_null());
        assert!(profiles.is_null() && lengths.is_null() && count == 0);
        unsafe { crate::idevice_error_free(error) };
    }
    #[test]
    fn exported_buffers_roundtrip_and_free() {
        let mut profiles = null_mut(); let mut lengths = null_mut(); let mut count = 0;
        let error = unsafe { export_profiles(Ok(vec![vec![1,2], vec![3]]), &mut profiles, &mut lengths, &mut count) };
        assert!(error.is_null()); assert_eq!(count, 2);
        unsafe {
            assert_eq!(std::slice::from_raw_parts(*profiles, *lengths), &[1,2]);
            assert_eq!(std::slice::from_raw_parts(*profiles.add(1), *lengths.add(1)), &[3]);
            misagent_free_profiles(profiles, lengths, count);
        }
    }
}
