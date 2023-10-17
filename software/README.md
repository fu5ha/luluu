# LuLuu! software

LuLuu's software stack is based on the Rust embedded and rp2040 tooling. 

The firmware is split into a couple of crates housed in the root Cargo workspace.
We provide our own "board support crate", [`luluu-bsp`](./luluu-bsp), and leverage
that crate in the actual firmware, contained in the [`luluu`](./luluu) folder.

TODO. Using Rust embedded crates:

- `embedded-hal` (`rp2040-hal`)
- `embedded-sdmmc`
- `embedded-graphics`
- `tinybmp`/`tinygif`
- `display-interface`
- `mipidsi`

## Building and installing

In order to build the firmware yourself, you'll need to:

1. Install the latest Rust and the `thumbv6m-none-eabi` target. First,
[install `rustup`](https://rustup.rs/). Then,

```
rustup self update
rustup update stable
rustup target add thumbv6m-none-eabi
```

2. Install [`flip-link`](https://github.com/knurling-rs/flip-link)

```
cargo install flip-link
```

Then you can build the firmware with `cargo build --release`. In order to install
the built firmware on the LuLuu unit, you have two options.

### Installing with `elf2uf2-rs`

This does not require an ARM 2-wire SWD capable probe, but is a bit cumbersome for
live development and does not provide debugging capabilities.

1. Install [`elf2uf2-rs`](https://github.com/JoNil/elf2uf2-rs)

```
cargo install elf2uf2-rs	
```

2. Change `.cargo/config` to use `elf2uf2-rs` as runner

Find `[target.thumbv6m-none-eabi]` and change it like so:

```toml
[target.thumbv6m-none-eabi]
runner = "elf2uf2-rs -d"
```

3. Put the LuLuu into "USB Bootloader mode". Do this by connecting the unit
via USB to your computer, then hold `USBBOOT` button and press the `RESET` button.
If on Linux, you may need to manually mount the device like you would a USB storage
device.

4. Run `cargo run --release` to build and install the firmware to the device

```
cargo run --release	
```

### Installing with `cargo embed`

This requires a [`probe-rs`-compatible probe](https://probe.rs/docs/getting-started/probe-setup/).
For example, a [RaspberryPi Debug Probe](https://www.raspberrypi.com/products/debug-probe/)
which LuLuu was designed to work with.

1. Install [`probe-rs`](https://crates.io/crates/probe-rs)

You may need to install prerequisites as well;
[see this page](https://probe.rs/docs/getting-started/installation/). If you're
on Linux, you may also need to change your `udev` rules, as specified
[here](https://probe.rs/docs/getting-started/probe-setup/).

```
cargo install probe-rs --features cli
```

2. Connect the LuLuu unit to power (i.e. plug it into a USB port; the port will
not be used for data in this configuration) as well as through its SWD port to
your probe.

3. Customize `Embed.toml` if desired.

See [the `probe-rs` repo for reference](https://github.com/probe-rs/probe-rs/blob/c0610e98008cbb34d0dc056fcddff0f2d4f50ad5/probe-rs/src/bin/probe-rs/cmd/cargo_embed/config/default.toml).

4. Run `cargo embed` to execute the `probe-rs` session

This compile, flash the device through the probe, and start running a debug session
according to the `Embed.toml`.

```
cargo embed
```

