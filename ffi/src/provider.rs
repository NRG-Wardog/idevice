// Jackson Coxson

use idevice::provider::IdeviceProvider;
#[cfg(feature = "tcp")]
use idevice::provider::TcpProvider;
#[cfg(feature = "usbmuxd")]
use idevice::provider::UsbmuxdProvider;
#[cfg(any(feature = "tcp", feature = "usbmuxd"))]
use std::ffi::CStr;
#[cfg(any(feature = "tcp", feature = "usbmuxd"))]
use std::os::raw::c_char;
use std::ptr::null_mut;

#[cfg(feature = "usbmuxd")]
use crate::usbmuxd::UsbmuxdAddrHandle;
#[cfg(feature = "tcp")]
use crate::util;
#[cfg(feature = "tcp")]
use crate::util::{SockAddr, idevice_sockaddr};
use crate::{IdeviceFfiError, ffi_err};
use crate::{IdevicePairingFile, run_sync};

pub struct IdeviceProviderHandle(pub Box<dyn IdeviceProvider>);

/// Creates a TCP provider for idevice
///
/// # Arguments
/// * [`ip`] - The sockaddr IP to connect to
/// * [`pairing_file`] - The pairing file handle to use
/// * [`label`] - The label to use with the connection
/// * [`provider`] - A pointer to a newly allocated provider
///
/// # Returns
/// An IdeviceFfiError on error, null on success
///
/// # Safety
/// `ip` must be a valid sockaddr
/// `pairing_file` is consumed must never be used again
/// `label` must be a valid Cstr
/// `provider` must be a valid, non-null pointer to a location where the handle will be stored
#[cfg(feature = "tcp")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idevice_tcp_provider_new(
    ip: *const idevice_sockaddr,
    pairing_file: *mut crate::pairing_file::IdevicePairingFile,
    label: *const c_char,
    provider: *mut *mut IdeviceProviderHandle,
) -> *mut IdeviceFfiError {
    if ip.is_null() || pairing_file.is_null() || label.is_null() || provider.is_null() {
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }
    unsafe { *provider = null_mut() };
    let ip = ip as *const SockAddr;
    let (addr, scope_id) = match util::c_addr_to_rust(ip) {
        Ok(i) => i,
        Err(e) => return ffi_err!(e),
    };

    let label = match unsafe { CStr::from_ptr(label).to_str() } {
        Ok(s) => s.to_string(),
        Err(_) => return ffi_err!(IdeviceError::FfiInvalidString),
    };

    // consume the pairing file on success
    let pairing_file = unsafe { Box::from_raw(pairing_file) };

    let t = TcpProvider {
        addr,
        scope_id,
        pairing_file: pairing_file.0,
        label,
    };

    let boxed = Box::new(IdeviceProviderHandle(Box::new(t)));
    unsafe { *provider = Box::into_raw(boxed) };
    std::ptr::null_mut()
}

/// Frees an IdeviceProvider handle
///
/// # Arguments
/// * [`provider`] - The provider handle to free
///
/// # Safety
/// `provider` must be a valid pointer to a IdeviceProvider handle that was allocated this library
///  or NULL (in which case this function does nothing)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idevice_provider_free(provider: *mut IdeviceProviderHandle) {
    if !provider.is_null() {
        tracing::debug!("Freeing provider");
        unsafe { drop(Box::from_raw(provider)) };
    }
}

/// Creates a usbmuxd provider for idevice
///
/// # Arguments
/// * [`addr`] - The UsbmuxdAddr handle to connect to
/// * [`tag`] - The tag returned in usbmuxd responses
/// * [`udid`] - The UDID of the device to connect to
/// * [`device_id`] - The muxer ID of the device to connect to
/// * [`label`] - The label to use with the connection
/// * [`provider`] - A pointer to a newly allocated provider
///
/// # Returns
/// An IdeviceFfiError on error, null on success
///
/// # Safety
/// `addr` must be a valid pointer to UsbmuxdAddrHandle created by this library, and never used again
/// `udid` must be a valid CStr
/// `label` must be a valid Cstr
/// `provider` must be a valid, non-null pointer to a location where the handle will be stored
#[cfg(feature = "usbmuxd")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn usbmuxd_provider_new(
    addr: *mut UsbmuxdAddrHandle,
    tag: u32,
    udid: *const c_char,
    device_id: u32,
    label: *const c_char,
    provider: *mut *mut IdeviceProviderHandle,
) -> *mut IdeviceFfiError {
    if addr.is_null() || udid.is_null() || label.is_null() || provider.is_null() {
        return ffi_err!(IdeviceError::FfiInvalidArg);
    }

    let udid = match unsafe { CStr::from_ptr(udid) }.to_str() {
        Ok(u) => u.to_string(),
        Err(e) => {
            tracing::error!("Invalid UDID string: {e:?}");
            return ffi_err!(IdeviceError::FfiInvalidString);
        }
    };

    let label = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(l) => l.to_string(),
        Err(e) => {
            tracing::error!("Invalid label string: {e:?}");
            return ffi_err!(IdeviceError::FfiInvalidArg);
        }
    };

    let addr = unsafe { Box::from_raw(addr) }.0;

    let p = UsbmuxdProvider {
        addr,
        tag,
        udid,
        device_id,
        label,
    };

    let boxed = Box::new(IdeviceProviderHandle(Box::new(p)));
    unsafe { *provider = Box::into_raw(boxed) };

    null_mut()
}

/// Gets the pairing file for the device
///
/// # Arguments
/// * [`provider`] - A pointer to the provider
/// * [`pairing_file`] - A pointer to the newly allocated pairing file
///
/// # Returns
/// An IdeviceFfiError on error, null on success
///
/// # Safety
/// `provider` must be a valid, non-null pointer to the provider
#[unsafe(no_mangle)]
pub unsafe extern "C" fn idevice_provider_get_pairing_file(
    provider: *mut IdeviceProviderHandle,
    pairing_file: *mut *mut IdevicePairingFile,
) -> *mut IdeviceFfiError {
    let provider = unsafe { &mut *provider };

    let res = run_sync(async move { provider.0.get_pairing_file().await });
    match res {
        Ok(pf) => {
            let pf = Box::new(IdevicePairingFile(pf));
            unsafe { *pairing_file = Box::into_raw(pf) };
            null_mut()
        }
        Err(e) => ffi_err!(e),
    }
}

#[cfg(test)]
mod ownership_tests {
    use super::*;
    use std::{future::Future, pin::Pin, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
    use idevice::{Idevice, IdeviceError, pairing_file::PairingFile};

    #[derive(Debug)]
    struct FailingProvider(Arc<AtomicUsize>);
    impl Drop for FailingProvider {
        fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); }
    }
    impl IdeviceProvider for FailingProvider {
        fn connect(&self, _: u16) -> Pin<Box<dyn Future<Output = Result<Idevice, IdeviceError>> + Send>> {
            Box::pin(async { Err(IdeviceError::UnexpectedResponse("controlled fixture".into())) })
        }
        fn label(&self) -> &str { "ownership-test" }
        fn get_pairing_file(&self) -> Pin<Box<dyn Future<Output = Result<PairingFile, IdeviceError>> + Send>> {
            Box::pin(async { Err(IdeviceError::UnexpectedResponse("controlled fixture".into())) })
        }
    }

    macro_rules! borrowed_provider_test {
        ($name:ident, $connect:path) => {
            #[test]
            fn $name() {
                let drops = Arc::new(AtomicUsize::new(0));
                let provider = Box::into_raw(Box::new(IdeviceProviderHandle(
                    Box::new(FailingProvider(drops.clone())),
                )));
                let null_output_error = unsafe { $connect(provider, null_mut()) };
                assert!(!null_output_error.is_null());
                assert_eq!(drops.load(Ordering::SeqCst), 0);
                unsafe { crate::idevice_error_free(null_output_error) };
                for _ in 0..2 {
                    let mut output = null_mut();
                    let error = unsafe { $connect(provider, &mut output) };
                    assert!(!error.is_null());
                    assert!(output.is_null());
                    assert_eq!(drops.load(Ordering::SeqCst), 0);
                    unsafe { crate::idevice_error_free(error) };
                }
                unsafe { idevice_provider_free(provider) };
                assert_eq!(drops.load(Ordering::SeqCst), 1);
            }
        }
    }
    borrowed_provider_test!(lockdown_errors_preserve_provider, crate::lockdown::lockdownd_connect);
    #[cfg(feature = "mobile_image_mounter")]
    borrowed_provider_test!(image_mounter_errors_preserve_provider, crate::mobile_image_mounter::image_mounter_connect);
    #[cfg(feature = "syslog_relay")]
    borrowed_provider_test!(os_trace_errors_preserve_provider, crate::os_trace_relay::os_trace_relay_connect);
    #[cfg(feature = "syslog_relay")]
    borrowed_provider_test!(syslog_errors_preserve_provider, crate::syslog_relay::syslog_relay_connect_tcp);
    #[cfg(feature = "springboardservices")]
    borrowed_provider_test!(springboard_errors_preserve_provider, crate::springboardservices::springboard_services_connect);
    #[cfg(feature = "house_arrest")]
    borrowed_provider_test!(house_arrest_errors_preserve_provider, crate::house_arrest::house_arrest_client_connect);
}
