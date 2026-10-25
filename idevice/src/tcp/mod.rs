// Jackson Coxson

pub use jktcp::adapter;
pub use jktcp::handle;
pub use jktcp::packets;
pub use jktcp::stream;

use crate::{ReadWrite, provider::RsdProvider};

impl RsdProvider for handle::AdapterHandle {
    async fn connect_to_service_port(
        &mut self,
        port: u16,
    ) -> Result<Box<dyn ReadWrite>, crate::IdeviceError> {
        let s = self.connect(port).await?;
        Ok(Box::new(s))
    }
}


/// Bound the generic IPv6 tunnel MTU before removing its 60-byte overhead.
/// Small or malformed MTUs saturate rather than underflow. Callers with a
/// stricter transport-specific cap (SideStore's 1340 MSS) retain that policy.
pub fn bounded_tunnel_mss(mtu: usize) -> usize {
    mtu.min(8192).saturating_sub(60)
}

#[cfg(test)]
mod mss_tests {
    use super::bounded_tunnel_mss;

    #[test]
    fn generic_tunnel_mss_is_bounded_without_small_mtu_underflow() {
        for (mtu, expected) in [(0, 0), (59, 0), (60, 0), (61, 1),
                                (1400, 1340), (8192, 8132), (8193, 8132), (usize::MAX, 8132)] {
            let mss = bounded_tunnel_mss(mtu);
            assert_eq!(mss, expected);
            assert!(mss <= mtu.saturating_sub(60));
            assert!(mss <= 8132);
        }
    }
}
