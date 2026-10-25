"""Execute maintained C++ wrappers against a deterministic consuming FFI double."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CXX = shutil.which('c++') or shutil.which('clang++')

class OwnershipTests(unittest.TestCase):
    @unittest.skipUnless(CXX, 'C++ compiler unavailable')
    def test_house_arrest_consumption_and_provider_borrow(self):
        with tempfile.TemporaryDirectory() as directory:
            binary = Path(directory) / 'ownership'
            subprocess.run([CXX, '-std=c++17', '-Wall', '-Wextra', '-Werror',
                '-I' + str(ROOT / '.ci/issue53/fixtures'), '-I' + str(ROOT / 'cpp/include'),
                str(ROOT / '.ci/issue53/fixtures/ownership.cpp'),
                str(ROOT / 'cpp/src/house_arrest.cpp'), str(ROOT / 'cpp/src/ffi.cpp'),
                '-o', str(binary)], check=True, capture_output=True, text=True)
            result = subprocess.run([str(binary)], check=True, capture_output=True, text=True)
            self.assertIn('OWNERSHIP_PASS', result.stdout)

    def test_upstream_provider_ownership_pairs(self):
        for name in ('lockdown', 'os_trace_relay', 'springboardservices', 'syslog_relay'):
            self.assertNotIn('provider.release()', (ROOT / f'cpp/src/{name}.cpp').read_text())
        for name in ('lockdown', 'mobile_image_mounter', 'os_trace_relay', 'springboardservices', 'syslog_relay'):
            source = (ROOT / f'ffi/src/{name}.rs').read_text()
            self.assertNotIn('Box::from_raw(provider)', source)
            self.assertIn('if provider.is_null() || client.is_null()', source)

    def test_usbmuxd_moves_box_field_without_read_then_drop(self):
        source = (ROOT / 'ffi/src/usbmuxd.rs').read_text()
        body = source.split('fn idevice_usbmuxd_connect_to_device(', 1)[1].split('/// Reads the pairing record', 1)[0]
        self.assertIn('Box::from_raw(usbmuxd_connection) }.0', body)
        self.assertNotIn('std::ptr::read', body)
        self.assertIn('label.is_null() || idevice.is_null()', body)

    def test_image_mounter_plists_borrow_wrapper_at_all_five_boundaries(self):
        source = (ROOT / 'ffi/src/mobile_image_mounter.rs').read_text()
        self.assertNotIn('Box::from_raw(info_plist', source)
        self.assertIn('&mut *(info_plist as *mut PlistWrapper)', source)
        self.assertIn('.borrow_self().clone()', source)
        self.assertEqual(source.count('let info_plist = unsafe { clone_borrowed_info_plist(info_plist) };'), 5)
        for name in ('mount_image', 'mount_personalized', 'mount_personalized_rsd',
                     'mount_personalized_with_callback', 'mount_personalized_with_callback_rsd'):
            body = source.split('fn image_mounter_' + name + '(', 1)[1].split('\n#[', 1)[0]
            self.assertLess(body.index('clone_borrowed_info_plist(info_plist)'), body.index('run_sync_local('))
        self.assertIn('caller_owned_wrapper_survives_repeated_independent_clones', source)
        self.assertIn('caller_retains_single_heap_owner_after_clone_is_dropped', source)

    def test_cryptex_install_uses_exact_distinct_persistence_fields(self):
        source = (ROOT / 'idevice/src/services/cryptexd.rs').read_text()
        self.assertIn('let mut argv = request.install_arguments();', source)
        helper = source.split('fn install_arguments(&self)', 1)[1].split('/// A request', 1)[0]
        self.assertIn('"persistence": XPCObject::UInt64(self.persistence)', helper)
        self.assertIn('"nonce-persistence": XPCObject::UInt64(self.nonce_persistence)', helper)
        self.assertNotIn('"persistence": XPCObject::UInt64(self.nonce_persistence)', helper)
        self.assertIn('custom_persistence_is_not_replaced_by_nonce_persistence', source)

if __name__ == '__main__':
    unittest.main(verbosity=2)
