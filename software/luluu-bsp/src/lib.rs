#![no_std]

use embedded_graphics::framebuffer::buffer_size;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::pixelcolor::raw::LittleEndian;
use embedded_graphics::prelude::PixelColor;
use embedded_sdmmc::{TimeSource, Timestamp};
pub use rp2040_hal as hal;

#[cfg(feature = "rt")]
use cortex_m_rt as _;
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

use hal::gpio::{Pin, FunctionUart, FunctionI2c, FunctionSpi, FunctionSioInput, FunctionSioOutput, FunctionPwm, PullUp, PullNone};
use hal::gpio::bank0::*;

pub type UartTx = Pin<Gpio0, FunctionUart, PullNone>;

pub type UartRx = Pin<Gpio1, FunctionUart, PullNone>;

pub type I2cData = Pin<Gpio2, FunctionI2c, PullUp>;

pub type I2cClock = Pin<Gpio3, FunctionI2c, PullUp>;

pub type SpiMiso = Pin<Gpio8, FunctionSpi, PullNone>;

pub type SpiMosi = Pin<Gpio15, FunctionSpi, PullNone>;

pub type SpiClock = Pin<Gpio14, FunctionSpi, PullNone>;

pub type CardCs = Pin<Gpio5, FunctionSioOutput, PullUp>;

pub type DispVsync = Pin<Gpio16, FunctionSioInput, PullNone>;

pub type DispReset = Pin<Gpio17, FunctionSioOutput, PullUp>;

pub type DispDataCmd = Pin<Gpio18, FunctionSioOutput, PullUp>;

pub type DispCsMain = Pin<Gpio19, FunctionSioOutput, PullUp>;

pub type DispBacklightToggle = Pin<Gpio22, FunctionSioOutput, PullUp>;

pub type DispBacklightPwm = Pin<Gpio22, FunctionPwm, PullUp>;

pub struct Pins {
    /// UART Tx pin
    pub uart_tx: UartTx,

    /// UART Rx pin
    pub uart_rx: UartRx,

    /// I2C Data pin
    pub i2c_data: I2cData,

    /// I2C Clock pin
    pub i2c_clock: I2cClock,

    /// Spi MISO/Rx pin
    pub spi_miso: SpiMiso,

    /// Spi MOSI/Tx pin
    pub spi_mosi: SpiMosi,

    /// Spi Clock pin
    pub spi_clock: SpiClock,

    /// microSD Card chip select (active low) pin
    pub card_cs: CardCs,

    /// Display VSYNC / TE / Tearing Effect pin
    pub disp_vsync: DispVsync,

    /// Display Reset (active low) pin
    pub disp_reset: DispReset,

    /// Display Data (high) / Command (low) (aka RS or DC) pin
    pub disp_data_cmd: DispDataCmd,

    /// Display main chip select (active low) pin
    pub disp_cs_main: DispCsMain,

    /// Display backlight pin.
    ///
    /// Default configured as [`DispBacklightToggle`] but can be reconfigured as [`DispBacklightPwm`]:
    ///
    /// ```ignore
    /// let bl_pin: DispBacklightPwm = pins.disp_backlight.reconfigure();
    /// ```
    pub disp_backlight: DispBacklightToggle,
}

impl Pins {
    pub fn new(
        io: hal::pac::IO_BANK0,
        pads: hal::pac::PADS_BANK0,
        sio: hal::sio::SioGpioBank0,
        reset: &mut hal::pac::RESETS,
    ) -> Self {
        let pins = hal::gpio::Pins::new(io, pads, sio, reset);
        Self {
            uart_tx: pins.gpio0.reconfigure(),
            uart_rx: pins.gpio1.reconfigure(),
            i2c_data: pins.gpio2.reconfigure(),
            i2c_clock: pins.gpio3.reconfigure(),
            spi_miso: pins.gpio8.reconfigure(),
            spi_mosi: pins.gpio15.reconfigure(),
            spi_clock: pins.gpio14.reconfigure(),
            card_cs: pins.gpio5.reconfigure(),
            disp_vsync: pins.gpio16.reconfigure(),
            disp_reset: pins.gpio17.reconfigure(),
            disp_data_cmd: pins.gpio18.reconfigure(),
            disp_cs_main: pins.gpio19.reconfigure(),
            disp_backlight: pins.gpio22.reconfigure(),
        }
    }
}

/// Layout of Spi pins.
pub type SpiPinLayout = (SpiMosi, SpiMiso, SpiClock);

/// A [`Framebuffer`][embedded_graphics::Framebuffer] type appropriate for the LuLuu!'s display module.
pub type Framebuffer = embedded_graphics::framebuffer::Framebuffer<
    Rgb565,
    <Rgb565 as PixelColor>::Raw,
    LittleEndian,
    240,
    320,
    { buffer_size::<Rgb565>(240, 320) }
>;

/// A dummy timesource, which is mostly important for creating files. Since we have no real-time
/// clock, just dummy it to the beginning of 2023.
#[derive(Default)]
pub struct DummyTimesource;

impl TimeSource for DummyTimesource {
    fn get_timestamp(&self) -> Timestamp {
        Timestamp {
            year_since_1970: 53,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

/// External oscillator frequency, 12Mhz is expected by the RP2040.
pub const XOSC_CRYSTAL_FREQ: u32 = 12_000_000;
