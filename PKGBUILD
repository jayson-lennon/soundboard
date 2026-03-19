pkgname=soundboard
pkgver=0.1.0
pkgrel=2
pkgdesc="Simple soundboard with varlink IPC for duckyPad"
arch=('x86_64')
url="https://github.com/example/soundboard"
license=('MIT')
depends=('gcc-libs' 'pipewire' 'pipewire-pulse')
makedepends=('cargo' 'rust')

# No source array needed - we reference files directly from $startdir
# This avoids conflicts with the project's src/ directory

# Dedicated build directory outside of project's src/ folder
_builddir="$startdir/.build/$pkgname-$pkgver"

prepare() {
    # Create dedicated build directory
    rm -rf "$_builddir"
    mkdir -p "$_builddir"

    # Copy Rust source files to build directory
    cp -r "$startdir/src" "$_builddir/"
    cp "$startdir/Cargo.toml" "$_builddir/"
    cp "$startdir/Cargo.lock" "$_builddir/"

    # Fetch dependencies in build directory
    cd "$_builddir"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$_builddir"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR="$_builddir/target"
    cargo build --frozen --release
}

package() {
    local _buildtarget="$_builddir/target/release"

    # Install binary
    install -Dm0755 -t "$pkgdir/usr/bin/" "$_buildtarget/soundboard"

    # Install udev rule
    install -Dm0644 -t "$pkgdir/usr/lib/udev/rules.d/" "$startdir/systemd/99-soundboard.rules"

    # Install systemd user units
    install -Dm0644 -t "$pkgdir/usr/lib/systemd/user/" "$startdir/systemd/soundboard.socket"
    install -Dm0644 -t "$pkgdir/usr/lib/systemd/user/" "$startdir/systemd/soundboard.service"

    # Install documentation
    install -Dm0644 -t "$pkgdir/usr/share/doc/soundboard/" "$startdir/config.example.toml"
}

post_install() {
    echo "Reloading udev rules..."
    udevadm control --reload-rules
    udevadm trigger
    echo "Reloading user systemd..."
    systemctl --user daemon-reload
    echo "Enabling soundboard units globally..."
    systemctl --global enable soundboard.socket soundboard.service
    echo ""
    echo "To enable soundboard:"
    echo "  1. Connect your duckyPad device"
    echo "  2. The socket will be activated automatically"
    echo "  3. Use 'soundboard play /path/to/audio.mp3' to play sounds"
}

post_remove() {
    systemctl --global disable soundboard.socket soundboard.service
    udevadm control --reload-rules
    systemctl --user daemon-reload
}
