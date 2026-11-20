#!/usr/bin/env bash
# Type-check every target of every workspace package for Linux and Windows from this Mac.
#
# TEST-SUPPORT mechanism 7 (docs/plans/TEST-SUPPORT/DESIGN.md#other-platforms): libraries, binaries,
# examples, tests and benches, default features, for x86_64-unknown-linux-gnu and x86_64-pc-windows-msvc.
# Nothing is linked. The workspace's C and C++ build scripts cannot cross-compile from a Mac, so they get
# stand-in compilers that write empty outputs, and bindgen gets the SDK's libc++ headers with Apple's
# availability markup turned off. Warnings do not fail the check; an error for either target does.
# Every stand-in lives in a temporary folder that is removed on exit, so a run leaves the checkout clean.
#
# Needs: rustup target add x86_64-unknown-linux-gnu x86_64-pc-windows-msvc
set -u

targets=(x86_64-unknown-linux-gnu x86_64-pc-windows-msvc)
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

for t in "${targets[@]}"; do
  if ! rustup target list --installed | grep -qx "$t"; then
    echo "check_cross_target: the $t standard library is not installed; run: rustup target add $t" >&2
    exit 2
  fi
done

shims=$(mktemp -d "${TMPDIR:-/tmp}/thrum-cross-target.XXXXXX")
trap 'rm -rf "$shims"' EXIT

# A compiler that writes an empty file wherever it was asked for output (-o FILE, or MSVC's -Fo/Fo).
cat >"$shims/fakecc" <<'EOF'
#!/usr/bin/env bash
out=""; prev=""
for a in "$@"; do
  if [ "$prev" = "-o" ]; then out="$a"; fi
  case "$a" in -Fo*|/Fo*) out="${a:3}";; esac
  prev="$a"
done
if [ -n "$out" ]; then : >"$out"; fi
exit 0
EOF
# An archiver that writes an empty archive.
cat >"$shims/fakear" <<'EOF'
#!/usr/bin/env bash
for a in "$@"; do
  case "$a" in
    -out:*|/OUT:*|/out:*) : >"${a#*:}"; exit 0;;
    *.a|*.lib) : >"$a"; exit 0;;
  esac
done
exit 0
EOF
chmod +x "$shims/fakecc" "$shims/fakear"
for n in x86_64-linux-gnu-gcc x86_64-linux-gnu-g++ ml64.exe; do ln -s "$shims/fakecc" "$shims/$n"; done

sdk=$(xcrun --show-sdk-path)
mkdir -p "$shims/libcxx"
sed 's/^#define _LIBCPP_HAS_VENDOR_AVAILABILITY_ANNOTATIONS 1$/#define _LIBCPP_HAS_VENDOR_AVAILABILITY_ANNOTATIONS 0/' \
  "$sdk/usr/include/c++/v1/__config_site" >"$shims/libcxx/__config_site"

# Use rustup's toolchain (the one rust-toolchain.toml names), which has the two targets' standard libraries,
# ahead of any other rustc on PATH, and no compiler cache.
toolchain_bin=$(dirname "$(rustup which rustc)")
export PATH="$toolchain_bin:$shims:$PATH" RUSTC_WRAPPER=
export CC_x86_64_unknown_linux_gnu="$shims/fakecc" CXX_x86_64_unknown_linux_gnu="$shims/fakecc"
export AR_x86_64_unknown_linux_gnu="$shims/fakear"
export CC_x86_64_pc_windows_msvc="$shims/fakecc" CXX_x86_64_pc_windows_msvc="$shims/fakecc"
export AR_x86_64_pc_windows_msvc="$shims/fakear"
export BINDGEN_EXTRA_CLANG_ARGS="-isystem $shims/libcxx -isystem $sdk/usr/include/c++/v1 -isystem $sdk/usr/include -Wno-everything"

status=0
for t in "${targets[@]}"; do
  echo "== $t"
  if ! cargo check --keep-going --locked --workspace --all-targets --target "$t" --message-format short; then
    echo "check_cross_target: $t has errors" >&2
    status=1
  fi
done
exit $status
