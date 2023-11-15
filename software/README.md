# LuLuu! software

LuLuu's software stack is based on the Rust embedded and rp2040 tooling.

The firmware is split into a couple of crates housed in the root Cargo workspace.
We provide our own "board support crate", [`luluu-bsp`](./luluu-bsp), and leverage
that crate in the actual firmware, contained in the [`luluu`](./luluu) folder.

There is also `luluu-enc`, a custom animated image encoding format for the LuLuu, and
`luluu-cli` whose main purpose is to convert `.GIF`s into `.LU`s which can be read and
displayed by the devide.

Using Rust embedded crates:

- `rp2040-hal`
- `embedded-hal`
- `embedded-sdmmc-rs`
- `display-interface`
- `mipidsi`

## Building and flashing the `luluu` firmware

In order to build the firmware yourself, you'll need to:


1. Initialize and update git submodules

We vendor some of our dependencies under the `vendored` directory. Because of this, you'll need
to init and update git submodules in order to build.

```
git submodule update --init
```

2. Install the latest Rust and the `thumbv6m-none-eabi` target. First,
[install `rustup`](https://rustup.rs/). Then,

```
rustup self update
rustup update stable
rustup target add thumbv6m-none-eabi
```

3. Install [`flip-link`](https://github.com/knurling-rs/flip-link)

```
cargo install flip-link
```

Then you can build the firmware by changing into the `luluu` directory and using
`cargo build --release`. In order to install the built firmware on the LuLuu unit,
you have two options.

### Installing with `elf2uf2-rs`

This does not require an ARM 2-wire SWD capable probe, but is a bit cumbersome for
live development and does not provide debugging capabilities.

1. Install [`elf2uf2-rs`](https://github.com/JoNil/elf2uf2-rs)

```
cargo install elf2uf2-rs
```

2. Change directory to the `luluu` directory within this `software` directory

```
cd luluu
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

3. Change directory to the `luluu` directory within this `software` directory

```
cd luluu
```

4. Run `cargo embed` to execute the `probe-rs` session

This compile, flash the device through the probe, and start running a debug session
according to `Embed.toml`.

To flash the fully release, no logging version:

```
cargo embed --release
```

To flash a release build but with logging enabled and start a defmt log console

```
cargo embed probe --release --features probe
```

This selects the "probe" profile from `Embed.toml` and enables the `probe` cargo feature
on the `luluu` crate, which enables its logging functionalities.

You can enable more or less verbose logging by modifying the `DEFMT_LOG` env variable
and `luluu/Cargo.toml`, for example uncomment the `embedded-sdmmc-rs/defmt-log` line to
enable logging from the library used to communicate with the SD card:

```
DEFMT_LOG=embedded_sdmmc=trace,luluu=debug cargo embed probe --release --features probe
```

## Converting GIFs with `luluu-cli`

You can convert animated gifs that are 60x60 or 120x120px in size and <= 15 frames per second
into `.LU` files that are used by the device by using the `luluu-cli` crate.

1. Change into the `luluu-cli` directory

```
cd luluu-cli
```

2. Run the tool, specifying the convert subcommand and a path to the file.

```
cargo run --release convert [FILE_PATH]
```

The tool will write the output file with the same name directly next to the original GIF provided.

You can get more help with

```
cargo run --release -- -h
```

or

```
cargo run --release -- convert -h
```
