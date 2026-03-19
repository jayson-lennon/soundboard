pkgname=soundboard
pkgver=0.1.0
pkgrel=1
pkgdesc="Simple soundboard with varlink IPC for duckyPad"
arch=('x86_64')
url="https://github.com/example/soundboard"
license=('MIT')
depends=('gcc-libs' 'pipewire' 'pipewire-pulse')
makedepends=('cargo' 'rust')
source=("$pkgname-$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm0755 "target/release/soundboard" "$pkgdir/usr/bin/soundboard"
    install -Dm0644 "systemd/99-soundboard.rules" "$pkgdir/usr/lib/udev/rules.d/99-soundboard.rules"
    install -Dm0644 "systemd/soundboard.socket" "$pkgdir/usr/lib/systemd/user/soundboard.socket"
    install -Dm0644 "systemd/soundboard.service" "$pkgdir/usr/lib/systemd/user/soundboard.service"
    install -Dm0644 "config.example.toml" "$pkgdir/usr/share/doc/soundboard/config.example.toml"
}

post_install() {
    echo "Reloading udev rules..."
    udevadm control --reload-rules
    udevadm trigger
    echo "Reloading user systemd..."
    systemctl --user daemon-reload
    echo ""
    echo "To enable soundboard:"
    echo "  1. Connect your duckyPad device"
    echo "  2. The socket will be activated automatically"
    echo "  3. Use 'soundboard play /path/to/audio.mp3' to play sounds"
}

post_remove() {
    udevadm control --reload-rules
    systemctl --user daemon-reload
}
