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

Three checks pass. The C++ harness compiles the actual maintained HouseArrest and error wrapper implementations using a minimal FFI double. It checks borrowed provider retention, vend success/failure, repeated use and exactly one destruction. The same harness fails against baseline 260caa0. This is not a Rust ABI integration test.

Rust tests were added to production modules but were not run locally because Rust/Cargo are unavailable. Required before integration:

```
cargo test --locked -p idevice --lib --no-default-features --features ring,usbmuxd
cargo test --locked -p idevice-ffi --lib --no-default-features --features ring,tcp,usbmuxd,house_arrest,mobile_image_mounter,syslog_relay,springboardservices
```

Retain the exact sibling jktcp source required by the existing path dependency. Run supported crypto/platform build configurations separately, regenerate C headers and compile real C++ consumers with those generated headers. Rust tests exercise short packet headers, valid raw packet round-trip, borrowed-provider repeated failures/null outputs, and exactly one socket drop after a consuming connection error.

## Deferred qualification

- The upstream u64 disconnect-ID change is not adopted here. It requires coordinated Rust, C header, C++ and caller qualification, including values above 32 bits. The current u32 truncation issue remains open.
- Adapter clone ownership, image-mounter borrowed plist lifetimes, tunnel MSS, cryptex persistence and Windows interface changes remain separate work. No changes to the custom jktcp override, lockfiles or 1340 MSS policy.
- OpenSSL stays pinned. Qualify both independent copies, provider activation, PKCS#12 fixtures, actual loaded images and signing before any bump. No security or pairing-fix claim is made.
- Additional C++ provider releases after borrowed connect errors remain in preboard_service, notification_proxy, companion_proxy, screenshotr, amfi, mobileactivationd, bt_packet_logger, pcapd, location_simulation, misagent, afc (two paths) and heartbeat. Their Rust connect paths appear to borrow providers; extend the consuming/borrowing runtime fixtures before changing these additional wrappers. WDA/WDA bridge consume providers and must not receive a blanket removal.
- Actual pairing, connect/disconnect, install, refresh and guest behavior still require device validation. Compilation is not runtime qualification.
