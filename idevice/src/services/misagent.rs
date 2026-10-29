//! iOS Mobile Installation Agent (misagent) Client
//!
//! Provides functionality for interacting with the misagent service on iOS devices,
//! which manages provisioning profiles and certificates.

use tracing::warn;

use crate::{Idevice, IdeviceError, IdeviceService, obf};

/// Client for interacting with the iOS misagent service
///
/// The misagent service handles:
/// - Installation of provisioning profiles
/// - Removal of provisioning profiles
/// - Querying installed profiles
#[derive(Debug)]
pub struct MisagentClient {
    /// The underlying device connection with established misagent service
    pub idevice: Idevice,
}

#[cfg(feature = "rsd")]
impl crate::RsdService for MisagentClient {
    fn rsd_service_name() -> std::borrow::Cow<'static, str> {
        obf!("com.apple.misagent.shim.remote")
    }

    async fn from_stream(stream: Box<dyn crate::ReadWrite>) -> Result<Self, IdeviceError> {
        let mut stream = Idevice::new(stream, "");
        stream.rsd_checkin().await?;
        Ok(Self::new(stream))
    }
}

impl IdeviceService for MisagentClient {
    /// Returns the misagent service name as registered with lockdownd
    fn service_name() -> std::borrow::Cow<'static, str> {
        obf!("com.apple.misagent")
    }

    async fn from_stream(idevice: Idevice) -> Result<Self, crate::IdeviceError> {
        Ok(Self::new(idevice))
    }
}

impl MisagentClient {
    /// Creates a new misagent client from an existing device connection
    ///
    /// # Arguments
    /// * `idevice` - Pre-established device connection
    pub fn new(idevice: Idevice) -> Self {
        Self { idevice }
    }

    /// Installs a provisioning profile on the device
    ///
    /// # Arguments
    /// * `profile` - The provisioning profile data to install
    ///
    /// # Returns
    /// `Ok(())` on successful installation
    ///
    /// # Errors
    /// Returns `IdeviceError` if:
    /// - Communication fails
    /// - The profile is invalid
    /// - Installation is not permitted
    ///
    /// # Example
    /// ```rust
    /// let profile_data = std::fs::read("profile.mobileprovision")?;
    /// client.install(profile_data).await?;
    /// ```
    pub async fn install(&mut self, profile: Vec<u8>) -> Result<(), IdeviceError> {
        let req = crate::plist!({
            "MessageType": "Install",
            "Profile": profile,
            "ProfileType": "Provisioning"
        });

        self.idevice.send_plist(req).await?;

        let mut res = self.idevice.read_plist().await?;

        match res.remove("Status") {
            Some(plist::Value::Integer(status)) => {
                if let Some(status) = status.as_unsigned() {
                    if status == 0 {
                        Ok(())
                    } else {
                        Err(IdeviceError::MisagentFailure)
                    }
                } else {
                    warn!("Misagent return status wasn't unsigned");
                    Err(IdeviceError::UnexpectedResponse(
                        "install Status is not an unsigned integer".into(),
                    ))
                }
            }
            _ => {
                warn!("Did not get integer status response");
                Err(IdeviceError::UnexpectedResponse(
                    "missing integer Status in install response".into(),
                ))
            }
        }
    }

    /// Removes a provisioning profile from the device
    ///
    /// # Arguments
    /// * `id` - The UUID of the profile to remove
    ///
    /// # Returns
    /// `Ok(())` on successful removal
    ///
    /// # Errors
    /// Returns `IdeviceError` if:
    /// - Communication fails
    /// - The profile doesn't exist
    /// - Removal is not permitted
    ///
    /// # Example
    /// ```rust
    /// client.remove("asdf").await?;
    /// ```
    pub async fn remove(&mut self, id: &str) -> Result<(), IdeviceError> {
        let req = crate::plist!({
            "MessageType": "Remove",
            "ProfileID": id,
            "ProfileType": "Provisioning"
        });

        self.idevice.send_plist(req).await?;

        let mut res = self.idevice.read_plist().await?;

        match res.remove("Status") {
            Some(plist::Value::Integer(status)) => {
                if let Some(status) = status.as_unsigned() {
                    if status == 0 {
                        Ok(())
                    } else {
                        Err(IdeviceError::MisagentFailure)
                    }
                } else {
                    warn!("Misagent return status wasn't unsigned");
                    Err(IdeviceError::UnexpectedResponse(
                        "remove Status is not an unsigned integer".into(),
                    ))
                }
            }
            _ => {
                warn!("Did not get integer status response");
                Err(IdeviceError::UnexpectedResponse(
                    "missing integer Status in remove response".into(),
                ))
            }
        }
    }

    /// Retrieves all provisioning profiles from the device
    ///
    /// # Returns
    /// A vector containing raw profile data for each installed profile
    ///
    /// # Errors
    /// Returns `IdeviceError` if:
    /// - Communication fails
    /// - The response is malformed
    ///
    /// # Example
    /// ```rust
    /// let profiles = client.copy_all().await?;
    /// for profile in profiles {
    ///     println!("Profile size: {} bytes", profile.len());
    /// }
    /// ```
    pub async fn copy_all(&mut self) -> Result<Vec<Vec<u8>>, IdeviceError> {
        let req = crate::plist!({
            "MessageType": "CopyAll",
            "ProfileType": "Provisioning"
        });

        self.idevice.send_plist(req).await?;

        let response = self.idevice.read_plist().await?;
        decode_copy_all_response(response)
    }
}

// Bounds apply before FFI duplicates buffers, not to the transport plist frame.
const MAX_PROFILE_COUNT: usize = 4096;
const MAX_PROFILE_BYTES: usize = 1_048_576;
const MAX_PROFILE_TOTAL_BYTES: usize = 33_554_432;

fn decode_copy_all_response(mut response: plist::Dictionary) -> Result<Vec<Vec<u8>>, IdeviceError> {
    match response.remove("Status") {
        Some(plist::Value::Integer(status)) if status.as_unsigned() == Some(0) => {}
        _ => return Err(IdeviceError::UnexpectedResponse("profile_readback_status_invalid".into())),
    }
    let Some(plist::Value::Array(payload)) = response.remove("Payload") else {
        return Err(IdeviceError::UnexpectedResponse("profile_readback_payload_invalid".into()));
    };
    if payload.len() > MAX_PROFILE_COUNT {
        return Err(IdeviceError::UnexpectedResponse("profile_readback_count_invalid".into()));
    }
    let mut profiles = Vec::with_capacity(payload.len());
    let mut total = 0usize;
    for value in payload {
        let plist::Value::Data(profile) = value else {
            return Err(IdeviceError::UnexpectedResponse("profile_readback_payload_invalid".into()));
        };
        if profile.is_empty() || profile.len() > MAX_PROFILE_BYTES
            || total > MAX_PROFILE_TOTAL_BYTES - profile.len() {
            return Err(IdeviceError::UnexpectedResponse("profile_readback_size_invalid".into()));
        }
        total += profile.len();
        profiles.push(profile);
    }
    Ok(profiles)
}

#[cfg(test)]
mod profile_readback_tests {
    use super::*;
    fn response(status: plist::Value, payload: plist::Value) -> plist::Dictionary {
        let mut result = plist::Dictionary::new();
        result.insert("Status".into(), status);
        result.insert("Payload".into(), payload);
        result
    }
    fn valid(payload: Vec<plist::Value>) -> plist::Dictionary {
        response(plist::Value::Integer(0.into()), plist::Value::Array(payload))
    }
    #[test]
    fn success_and_empty_store_are_observations() {
        assert_eq!(decode_copy_all_response(valid(vec![plist::Value::Data(vec![1,2])])).unwrap(), vec![vec![1,2]]);
        assert!(decode_copy_all_response(valid(vec![])).unwrap().is_empty());
    }
    #[test]
    fn status_is_mandatory_integer_zero() {
        for status in [plist::Value::Boolean(false), plist::Value::Boolean(true),
            plist::Value::Real(0.0), plist::Value::String("0".into()),
            plist::Value::Integer(1.into()), plist::Value::Integer((-1).into())] {
            assert!(decode_copy_all_response(response(status, plist::Value::Array(vec![]))).is_err());
        }
        let mut missing = valid(vec![]); missing.remove("Status");
        assert!(decode_copy_all_response(missing).is_err());
    }
    #[test]
    fn payload_must_be_complete_data_array() {
        let mut missing = valid(vec![]); missing.remove("Payload");
        assert!(decode_copy_all_response(missing).is_err());
        assert!(decode_copy_all_response(response(plist::Value::Integer(0.into()), plist::Value::String("bad".into()))).is_err());
        assert!(decode_copy_all_response(valid(vec![plist::Value::Data(vec![1]), plist::Value::Boolean(false)])).is_err());
        assert!(decode_copy_all_response(valid(vec![plist::Value::Data(vec![])])).is_err());
    }
    #[test]
    fn bounds_fail_closed_without_truncation() {
        assert!(decode_copy_all_response(valid(vec![plist::Value::Data(vec![1]); MAX_PROFILE_COUNT + 1])).is_err());
        assert!(decode_copy_all_response(valid(vec![plist::Value::Data(vec![1; MAX_PROFILE_BYTES + 1])])).is_err());
        assert!(decode_copy_all_response(valid(vec![plist::Value::Data(vec![1; MAX_PROFILE_BYTES]); 33])).is_err());
        assert_eq!(decode_copy_all_response(valid(vec![plist::Value::Data(vec![1; MAX_PROFILE_BYTES]); 32])).unwrap().len(), 32);
    }
}
