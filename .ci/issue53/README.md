# Selective dependency ownership candidate

This candidate applies the bounded provider and HouseArrest ownership corrections from upstream commit [33b8fb57](https://github.com/jkcoxson/idevice/commit/33b8fb57ae51c745d0a50d2321c99636b196089a), with null-output handling and regression coverage. It also prevents short usbmuxd packet underflow, related to [f36b36ed](https://github.com/jkcoxson/idevice/commit/f36b36ed36dbd56826cee3d1919fc9310cb66f33), rejects malformed raw-packet slice lengths, and fixes a separate confirmed consumed-connection double destruction.

## Ownership contract

- Lockdown, image mounter, OS trace, SpringBoard, syslog and HouseArrest connect calls borrow the provider on success and failure. C++ callers retain their owning handle.
- HouseArrest vend operations consume a valid client with valid arguments on success and service failure. C++ relinquishes ownership before the call. Null arguments do not consume the Rust handle.
- usbmuxd connect consumes a non-null connection once, including invalid label/output and service failure. Moving its field out of an owning Box must not first copy the field with `ptr::read` and then destroy the original owner.
- Public signatures and disconnect-ID ABI are unchanged. Generated headers must still be rebuilt with the candidate binary; no unverified prebuilt framework is substituted.

## Checks

Executed locally:

```
python -B .ci/issue53/test_ownership.py
```

Five checks pass (four source contracts and one C++ harness). The C++ harness compiles the actual maintained HouseArrest and error wrapper implementations using a minimal FFI double. It checks borrowed provider retention, vend success/failure, repeated use and exactly one destruction. The same harness fails against baseline 260caa0. This is not a Rust ABI integration test.

Rust tests were added to production modules but were not run locally because Rust/Cargo are unavailable. Required before integration:

```
cargo test --locked -p idevice --lib --no-default-features --features ring,usbmuxd
cargo test --locked -p idevice-ffi --lib --no-default-features --features ring,tcp,usbmuxd,house_arrest,mobile_image_mounter,syslog_relay,springboardservices
```

Retain the exact sibling jktcp source required by the existing path dependency. Run supported crypto/platform build configurations separately, regenerate C headers and compile real C++ consumers with those generated headers. Rust tests exercise short packet headers, valid raw packet round-trip, borrowed-provider repeated failures/null outputs, and exactly one socket drop after a consuming connection error.

## Deferred qualification

- The u64 disconnect-ID change now has paired Rust/FFI and generated-header checks in this candidate; native qualification remains required as detailed below.
- Adapter clone ownership remains separate work; the bounded MSS and Windows changes are detailed below. No changes to the custom jktcp override, lockfiles or 1340 MSS policy.
- OpenSSL stays pinned. Qualify both independent copies, provider activation, PKCS#12 fixtures, actual loaded images and signing before any bump. No security or pairing-fix claim is made.
- Additional C++ provider releases after borrowed connect errors remain in preboard_service, notification_proxy, companion_proxy, screenshotr, amfi, mobileactivationd, bt_packet_logger, pcapd, location_simulation, misagent, afc (two paths) and heartbeat. Their Rust connect paths appear to borrow providers; extend the consuming/borrowing runtime fixtures before changing these additional wrappers. WDA/WDA bridge consume providers and must not receive a blanket removal.
- Actual pairing, connect/disconnect, install, refresh and guest behavior still require device validation. Compilation is not runtime qualification.


## Additional bounded image-mounter and cryptex corrections

- Upstream `858d26352a77b1b061f2b672bf477c9a5eb5708d` fixes five
  image-mounter input plist lifetimes. The actual FFI object is `PlistWrapper`,
  not a bare `plist::Value`; all five calls now use one audited borrowed-wrapper
  clone helper before async work. C++ signatures remain unchanged and document
  caller ownership on success/failure. Null inputs remain optional.
- Upstream `ded303b8b3af9e3fdd2b562200c286ab00c79cce` sends the request's
  `persistence` field rather than copying `nonce_persistence`. The real install
  path now calls a pure argument constructor; transfers/wire fields otherwise
  remain unchanged. Tests distinguish default values 2/1 and custom u64 values.
- Five local source/C++ checks pass. Three actual Rust plist-lifetime tests and
  two pure serialization tests are added, but require Cargo execution in CI:

```
cargo test --locked -p idevice --lib --no-default-features --features ring,usbmuxd,cryptexd
cargo test --locked -p idevice-ffi --lib --no-default-features --features ring,tcp,usbmuxd,house_arrest,mobile_image_mounter,syslog_relay,springboardservices,tss,tunnel_tcp_stack
```

No Rust execution or actual device image mounting/cryptex persistence was
performed locally. No jktcp, MSS, lockfile, listener ABI or adapter changes are
included in these two corrections.


## Full-width listener and Windows argument spelling

Upstream `f36b36ed36dbd56826cee3d1919fc9310cb66f33` widens detach IDs and
marks the listener stream Send. The candidate carries u64 from parsed wire plist
through the actual C ABI output and clears valid outputs before polling, including
error/end-of-stream. Rust fixtures exercise zero, 2^32+7 and u64::MAX through the
real wire listener and the real FFI function with a synthetic stream. They do not
connect to usbmuxd or an Apple device. The wire fixture has bounded timeouts.

After Cargo regenerates both header copies, run:

```
python3 -B .ci/issue53/test_listener.py
bash .ci/issue53/check_listener_abi.sh
```

The C11 and C++17 checks compile against the actual freshly generated headers.
Local tests execute these same assertions against explicit synthetic new/old
headers to prove rejection of uint32_t*, not to claim the actual generated header
was built here. The SAR native ABI gate additionally checks this function's exact
uint64_t* signature in all three header copies and requires its exported symbol.

API/caller review searched `cpp`, `swift`, `ffi/examples` and all Rust sources for
`idevice_usbmuxd_listener_next`, disconnect outputs and `UsbmuxdListenEvent`.
No C++/Swift listener caller exists in these maintained trees; the only internal
consumer is the FFI bridge. **Old 32-bit headers must never be mixed with the new
library**, which writes eight bytes. Rebuild the Rust library, cbindgen headers,
C/C++ consumers and Swift-importing framework together. Device connect/disconnect
qualification is still pending.

Upstream `c8086a39622e29d5536085da4815fe29a14fed6b` renames only the recovery
trait argument `interface` to `iface`, avoiding a Windows macro collision. No
parameter type/order or behavior changes. Windows target compilation is not
claimed from Linux source controls or an iOS build.


## Bounded generic tunnel MSS

Upstream `edf25870f5107ffce074409ee4f230bdd7e73632` caps generic negotiated MTU
at 8192 before subtracting IPv6/TCP overhead. This candidate retains saturating
subtraction for MTUs below 60 instead of introducing upstream's underflow edge.
Only `CoreDeviceProxy::create_software_tunnel` and generic `finish_tunnel` use the
shared helper. The entire custom `tunnel_create_usb` implementation is byte-for-
byte green-baseline identical and still caps effective MSS at 1340. jktcp remains
the exact maintained 0.1.8 path dependency; no lockfiles or custom transport
policy were replaced.

`python3 -B .ci/issue53/test_mss.py` runs two source controls, including the full
custom USB path hash. Eight executable Rust boundary assertions (0, 59, 60, 61,
1400, 8192, 8193, usize::MAX) require `cargo test --locked -p idevice --lib
--no-default-features --features ring,tunnel_tcp_stack` in native CI. Actual
transport performance and device behavior are not proven by these arithmetic
checks.

## Conditional adapter cloning: deliberately not adopted

Upstream `26a28f6a46b5d85a28043c2e62febfa394c8b0a6` adds `adapter_clone` and
C++ copy construction/assignment. Issue #53 makes adoption conditional on an
explicit shared-lifetime/close contract. Inspection of exact maintained jktcp
`a46594f6af56b66b6d9342ed9f5ee6d1b7fb9cc9` (`src/handle.rs`, `src/adapter.rs`)
confirms a cloned channel sender keeps its background adapter alive, as do stream
senders; explicit `close()` sends `Die` and shuts down all copies.

That upstream contract is **incompatible with the current custom FFI teardown**:
`ffi/src/core_device_proxy.rs::adapter_free` unconditionally calls
`tunnel_heartbeat_stop`, and `ffi/src/tunnel_provider.rs` keeps a global heartbeat
task. Freeing one newly cloned FFI handle would therefore stop the heartbeat
needed by other handles and streams, even though jktcp's adapter stays alive.
A count of FFI wrappers alone would still be wrong when a stream outlives them.
No clone export or C++ copy operation is introduced in this candidate.

Future adoption needs a generation-owned heartbeat lease tied to the underlying
adapter transport/actor lifetime, including stream/RSD users. Releasing an old
handle or completing an old heartbeat task must not stop or mark a replacement
tunnel inactive. Required tests must cover original-first and clone-first drops,
streams outliving handles, explicit close stopping every copy, exactly-once final
cleanup and overlapping old/new tunnel generations using actual Rust lifecycle
code. This is a separately qualified lifecycle change, not a safe mechanical
cherry-pick. The existing single-handle API/teardown and 1340 MSS policy are
retained; this is an explicit conditional nonadoption, not a runtime success claim.

`test_adapter_clone_scope.py` guards against accidentally adding only half the
upstream cloning change. It is intentionally replaceable when the complete
shared-lifetime contract and executable tests above are implemented.

### Exact upstream comparison accounting

Comparison: `774f02a6d29777c85400f4ba7fa69c12b4227fb3...26a28f6a46b5d85a28043c2e62febfa394c8b0a6`
(diverged history, nine commits ahead; no wholesale merge).

| Upstream commit | Decision |
| --- | --- |
| `33b8fb57` provider/HouseArrest ownership | Already selected in green candidate; retained and tested. |
| `f36b36ed` packet/listener | Existing packet guards retained; full-width listener + Send and coordinated ABI checks added. |
| `858d2635` image-mounter plist | Borrowed wrapper clone adopted at all five boundaries. |
| `3854a5df` usbmuxd connection ownership | Already corrected in green candidate; retained with exactly-once drop fixture. |
| `edf25870` generic MSS cap | Adopted with saturating arithmetic; stricter custom 1340 MSS preserved. |
| `ded303b8` cryptex persistence | Request field corrected; distinct/default/custom scalar tests added. |
| `9c1ad299` jktcp 0.1.8 | Maintained pinned 0.1.8 path dependency already covers version intent; no lockfile rollback. |
| `c8086a39` Windows identifier | Exact `interface` to `iface` argument rename adopted; Windows runtime not claimed. |
| `26a28f6a` adapter cloning | Conditional nonadoption for the documented custom heartbeat ownership conflict. |
