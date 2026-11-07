# Xcode origin for the first macOS release-input manifest

3 November 2026. **Policy choice pending.** This narrows the selected
[release-input trust policy](../T2-NATIVE-RELEASE-TRUST.md) for the first
CPU-only macOS ARM64 development builder. It does not approve a release
manifest or make a build ready.

## Required boundary

T2 needs an independently reviewed exact-hash record for the selected linker,
SDK and support bytes. The record must explain why the first bytes are
accepted as an official Apple distribution, then hold the selected copied
roots against accidental and compiler writes through the complete build.
The current matcher already permits `apps.apple.com` and
`developer.apple.com` origin URLs, but checks only the record's shape and
local bytes; it cannot attest an acquisition job. F3 source and F4 product
identities remain separate, and a multiplayer match must pin a future ready
`BuildRevision`, never an unreviewed tool candidate.

Apple lists [Xcode 27.2 build 27B52](https://developer.apple.com/news/releases/?id=10292026a)
as released on 29 October 2026 and directs users to the
[Mac App Store](https://developer.apple.com/xcode/resources/) for Xcode.
Apple also offers [developer downloads](https://developer.apple.com/xcode/resources/)
after Apple Account sign-in. The [Xcode App Store listing](https://apps.apple.com/us/app/xcode/id497799835)
is an official Apple listing. These pages establish official distribution
channels and the named release; they do not provide an exact installer hash
for this machine's selected files.

## Current local evidence

On this Mac, `xcodebuild -version` reports **Xcode 27.2, 27B52**. The app has
a 4,858-byte `_MASReceipt/receipt`, which has not been independently decoded
or validated. Outside the command sandbox, `codesign --verify --deep --strict`
reported the app valid on disk and satisfying its designated requirement;
`codesign -dv` reported `com.apple.dt.Xcode`, team `59GAB85EFG`, Apple Mac OS
Application Signing and an Apple certificate chain. `spctl --assess --type
execute` accepted it with `source=Mac App Store`. The same assessment inside
the command sandbox returned an internal Code Signing subsystem error, so
the successful outside-sandbox result is the relevant observation. The
existing [local origin candidate](../../evidence/i2a/T2-RELEASE-ORIGIN-CANDIDATE.md)
records earlier matching observations. These commands did not capture the
original App Store download transaction or establish an Apple-published hash
for the installed Xcode bundle.

## Options

| Choice | Review input and preparation | Cost and remaining trust |
| --- | --- | --- |
| **A. Signed Mac App Store installation (recommended for this development profile)** | A controlled preparer snapshots the complete installed app into a private no-writer owner, then verifies signature, designated identity, Store assessment and receipt against that same snapshot before deriving and holding exact selected-root/support hashes. Human review approves the resulting manifest and preparation record. | Reuses the installed app and trusts macOS's App Store/signing assessment plus the local user. No independent downloaded installer byte hash; full receipt validation and successful signature/Store assessment of the frozen snapshot still need qualification. |
| **B. Apple Developer download archive** | A controlled preparer downloads the selected Xcode archive through the signed-in Apple Developer channel, records URL, archive SHA-256 and archive verification result, then inventories the expanded selected bytes. Human review approves the archive and root identities. | Requires a fresh multi-gigabyte download, expansion, disk capacity and a reviewable archive-verification procedure. A locally computed archive hash is only an identity unless Apple authentication of the downloaded archive is checked. The official pages inspected here do not supply a public expected archive SHA-256. |

Both choices still require exact Rust and Cargo origin proofs, Xcode support and
host-runtime read closure, one held no-writer selection through metadata and
output publication, reviewed helper-family behavior, and the ready-root
transaction. Neither choice changes the current `buildReady:false` status.

## Proposed next proof if A is selected

1. Implement a bounded Store-receipt validator using Apple's
   [complete on-device procedure](https://developer.apple.com/documentation/appstorereceipts/validating-receipts-on-the-device):
   verify the PKCS #7 signature and Apple chain at the signed receipt creation
   date, compare bundle ID and the receipt's app version with the signed app's
   `CFBundleShortVersionString`, and check the device-bound SHA-1 value. The
   receipt version is not Xcode's `27B52` build number. Record only the
   validation result, non-personal identities and a digest; do not retain
   receipt bytes or device identifiers in the repository.
2. Copy the **whole app, including receipt and code-signing resources**, into
   a private candidate snapshot, make it no-writer under the selected
   trusted-local-user policy, then run signature, Store assessment and receipt
   checks against that same snapshot. Derive linker/SDK/support bytes only
   from the authenticated snapshot while its lease remains held. A live-app
   signature check followed by a later root inventory is insufficient: an
   updater could change the live app between those operations. A race fixture
   must change the installed source during preparation and prove refusal or
   a still-authenticated exact snapshot.
3. Refuse a missing receipt, wrong bundle/version/team, failed signature or
   changed selected snapshot. Bind the accepted app identity and exact
   linker/SDK/support root hashes in the candidate release manifest. Review
   that exact manifest and the job
   record before allowing it to select a ready build. Keep cold inventory and
   receipt validation outside each edit; warm builds use the held sealed roots.

This proposal is limited to the trusted-local-user development profile.
No receipt verifier or approved manifest exists yet.
