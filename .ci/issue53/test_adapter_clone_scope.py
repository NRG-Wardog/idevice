"""Guard the explicit conditional nonadoption, not a permanent ban on cloning.
Replace alongside a qualified generation-owned heartbeat/stream lifetime design.
"""
from pathlib import Path
import unittest
ROOT = Path(__file__).resolve().parents[2]
class CloneScopeTests(unittest.TestCase):
    def test_no_partial_clone_adoption_over_global_heartbeat_teardown(self):
        ffi = (ROOT / 'ffi/src/adapter.rs').read_text()
        cpp = (ROOT / 'cpp/include/idevice++/core_device_proxy.hpp').read_text()
        free = (ROOT / 'ffi/src/core_device_proxy.rs').read_text().split('fn adapter_free(', 1)[1]
        self.assertNotIn('fn adapter_clone(', ffi, 'Cloning requires the owned heartbeat/stream lifetime design, not a partial cherry-pick')
        self.assertIn('Adapter(const Adapter&)', cpp)
        self.assertIn('Adapter(const Adapter&)                          = delete;', cpp)
        self.assertIn('operator=(const Adapter&) = delete;', cpp)
        self.assertIn('tunnel_heartbeat_stop()', free)
        self.assertIn('Box::from_raw(handle)', free)
        docs = (ROOT / '.ci/issue53/README.md').read_text()
        self.assertIn('Conditional adapter cloning: deliberately not adopted', docs)
        self.assertIn('streams outliving handles', docs)
if __name__ == '__main__': unittest.main(verbosity=2)
