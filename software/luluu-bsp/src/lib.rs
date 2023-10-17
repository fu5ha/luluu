#![no_std]

pub use rp2040_hal as hal;

#[cfg(feature = "rt")]
pub use cortex_m_rt as rt;
#[cfg(feature = "rt")]
pub use hal::entry;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
#[cfg(feature = "boot2")]
#[link_section = ".boot2"]
#[no_mangle]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_GD25Q64CS;

pub use hal::pac;

hal::bsp_pins!(
    /// UART TX
    Gpio0 {
        name: tx,
        aliases: { FunctionUart, PullNone: UartTx }
    },
    /// UART RX
    Gpio1 {
        name: rx,
        aliases: { FunctionUart, PullNone: UartRx }
    },
    /// I2C Data
    Gpio2 {
        name: sda,
        aliases: { FunctionI2C, PullUp: Sda }
    },
    /// I2C Clock
    Gpio3 {
        name: scl,
        aliases: { FunctionI2C, PullUp: Scl }
    },
    /// microSD Card Chip Select (active low)
    Gpio5 {
        name: card_cs,
        aliases: { FunctionSioOutput, PullUp: CardCs }
    },
    /// SPI Main In Sub Out
    Gpio8 {
        name: miso,
        aliases: { FunctionSpi, PullNone: Miso }
    },
    /// SPI Clock
    Gpio14 {
        name: sclk,
        aliases: { FunctionSpi, PullNone: Sclk }
    },
    /// SPI Main Out Sub In
    Gpio15 {
        name: mosi,
        aliases: { FunctionSpi, PullNone: Mosi }
    },
    /// Display VSYNC signal (aka TE / Tearing Effect)
    Gpio16 {
        name: disp_vsync,
        aliases: { FunctionSioInput, PullNone: DispVsync }
    },
    /// Display RESET (active low)
    Gpio17 {
        name: disp_reset,
        aliases: { FunctionSioOutput, PullUp: DispReset }
    },
    /// Display sub chip select Data (high) / Commands (low)
    Gpio18 {
        name: disp_cs_data_cmd,
        aliases: { FunctionSioOutput, PullNone: DispDataCmd }
    },
    /// Display main chip select (active low)
    Gpio19 {
        name: disp_cs_main,
        aliases: { FunctionSioOutput, PullUp: DispCsMain }
    },
    /// Display backlight
    Gpio22 {
        name: disp_backlight,
        aliases: {
            FunctionSioOutput, PullUp: DispBlToggle,
            FunctionPwm, PullNone: DispBlPwm
        }
    },
);

pub const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;
