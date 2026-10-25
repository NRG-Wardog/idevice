"""Source and synthetic-header controls; real Rust/header checks run in native CI."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]

class ListenerTests(unittest.TestCase):
    def test_real_source_keeps_u64_wire_and_ffi_contract(self):
        core = (ROOT / 'idevice/src/usbmuxd/mod.rs').read_text()
        ffi = (ROOT / 'ffi/src/usbmuxd.rs').read_text()
        self.assertIn('Disconnected(u64)', core)
        self.assertIn('UsbmuxdListenEvent::Disconnected(id)', core)
        self.assertNotIn('Disconnected(id as u32)', core)
        self.assertIn('disconnection_id: *mut u64', ffi)
        self.assertIn('*connect = false;', ffi)
        self.assertIn('*connection_device = null_mut();', ffi)
        self.assertIn('*disconnection_id = 0;', ffi)
        self.assertIn('real_listener_preserves_full_wire_disconnect_ids', core)
        self.assertIn('actual_ffi_writes_full_width_disconnect', ffi)

    def test_windows_trait_avoids_macro_name(self):
        source = (ROOT / 'idevice/src/services/restore/recovery/mod.rs').read_text()
        self.assertIn('claim_interface(&mut self, iface: u8', source)
        self.assertNotIn('claim_interface(&mut self, interface: u8', source)

    def test_c_and_cpp_checks_reject_old_header_pointer_width(self):
        for compiler, standard, filename in [('cc', 'c11', 'assert_listener_abi.c'), ('c++', 'c++17', 'assert_listener_abi.cpp')]:
            self.assertIsNotNone(shutil.which(compiler), 'Host compiler required for gate controls')
            with tempfile.TemporaryDirectory() as td:
                for width in ('uint64_t', 'uint32_t'):
                    Path(td, 'idevice.h').write_text('#include <stdint.h>\n#include <stdbool.h>\n' +
                        '\n'.join('typedef struct ' + n + ' ' + n + ';' for n in ['IdeviceFfiError', 'UsbmuxdListenerHandle', 'UsbmuxdDeviceHandle']) +
                        '\nIdeviceFfiError *idevice_usbmuxd_listener_next(UsbmuxdListenerHandle *, bool *, UsbmuxdDeviceHandle **, ' + width + ' *);\n')
                    result = subprocess.run([compiler, '-std=' + standard, '-Werror', '-fsyntax-only', '-I' + td,
                        str(ROOT / '.ci/issue53' / filename)], capture_output=True, text=True)
                    self.assertEqual(result.returncode == 0, width == 'uint64_t', result.stderr)

if __name__ == '__main__': unittest.main(verbosity=2)
