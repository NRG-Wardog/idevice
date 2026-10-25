// C++ consumers compile the same newly generated C header as Rust's library.
#include <cstdint>
#include <type_traits>
extern "C" {
#include <idevice.h>
}
using ListenerNext64 = IdeviceFfiError *(*)(UsbmuxdListenerHandle *, bool *, UsbmuxdDeviceHandle **, uint64_t *);
static_assert(std::is_same_v<decltype(&idevice_usbmuxd_listener_next), ListenerNext64>, "C++ listener ABI must use uint64_t*");
static_assert(UINT64_C(0x100000007) > UINT32_MAX, "fixture must cover IDs wider than 32 bits");
