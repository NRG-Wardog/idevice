"""Source-only MSS regression controls; actual Rust boundaries run in CI."""
from pathlib import Path
import hashlib
import unittest
ROOT = Path(__file__).resolve().parents[2]
class MssTests(unittest.TestCase):
    def test_only_generic_callers_use_bounded_helper(self):
        core = (ROOT / 'idevice/src/services/core_device_proxy.rs').read_text()
        ffi = (ROOT / 'ffi/src/tunnel_provider.rs').read_text()
        helper = (ROOT / 'idevice/src/tcp/mod.rs').read_text()
        self.assertIn('adapter.set_mss(crate::tcp::bounded_tunnel_mss(mtu));', core)
        self.assertIn('adapter.set_mss(idevice::tcp::bounded_tunnel_mss(mtu));', ffi)
        self.assertIn('mtu.min(8192).saturating_sub(60)', helper)
        self.assertNotIn('mtu.min(8192) - 60', helper)
        self.assertIn('(usize::MAX, 8132)', helper)
    def test_custom_usb_path_is_byte_identical_to_green(self):
        source = (ROOT / 'ffi/src/tunnel_provider.rs').read_text()
        custom = source.split('pub unsafe extern "C" fn tunnel_create_usb(', 1)[1].split('pub unsafe extern "C" fn tunnel_pair_usb(', 1)[0]
        self.assertEqual(hashlib.sha256(custom.encode()).hexdigest(), '01b1e9ab959364d39df4a273afafb69a29abbfd9462336f738662b96208a13cc')
        self.assertIn('negotiated_mtu.saturating_sub(60).min(1340)', custom)
        self.assertNotIn('bounded_tunnel_mss', custom)
if __name__ == '__main__': unittest.main(verbosity=2)
