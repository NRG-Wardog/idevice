// Compile against the freshly generated candidate header, never a vendored stub.
#include <stdint.h>
#include <stdbool.h>
#include <idevice.h>
typedef struct IdeviceFfiError *(*ListenerNext64)(struct UsbmuxdListenerHandle *, bool *, struct UsbmuxdDeviceHandle **, uint64_t *);
_Static_assert(_Generic(&idevice_usbmuxd_listener_next, ListenerNext64: 1, default: 0), "listener disconnect output ABI must be uint64_t*");
_Static_assert(sizeof(uint64_t) == 8, "64-bit disconnect storage required");
