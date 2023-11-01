#![no_std]
#![no_main]

use core::cell::RefCell;

use bsp::hal::i2c::peripheral;
use display_interface_spi::SPIInterface;
use embedded_hal_bus::spi::RefCellDevice;
use embedded_sdmmc::{VolumeIdx, ShortFileName};
use luluu_bsp as bsp;

use bsp::hal as hal;
use bsp::{entry, hal::Spi, SpiPinLayout, DispBacklightToggle, Framebuffer};
use defmt::*;
use defmt_rtt as _;
use embedded_hal::{digital::OutputPin, spi::MODE_0};
use embedded_sdmmc::sdcard::DummyCsPin;
use panic_probe as _;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

use fugit::RateExtU32;

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut peripherals = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(peripherals.WATCHDOG);
    let sio = Sio::new(peripherals.SIO);

    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        peripherals.XOSC,
        peripherals.CLOCKS,
        peripherals.PLL_SYS,
        peripherals.PLL_USB,
        &mut peripherals.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = hal::Timer::new(peripherals.TIMER, &mut peripherals.RESETS, &clocks);

    let pins = bsp::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    let mut backlight_pin = pins.disp_backlight;
    backlight_pin.set_low().unwrap();

    let card_cs = pins.card_cs;

    let disp_cs_main = pins.disp_cs_main;
    let disp_data_cmd = pins.disp_data_cmd;


    let spi_pin_layout: SpiPinLayout = (pins.spi_mosi, pins.spi_miso, pins.spi_clock);
    let spi = Spi::new(peripherals.SPI1, spi_pin_layout);

    // start at low baud rate for initializing card
    let spi = spi.init(&mut peripherals.RESETS, 125.MHz(), 400.kHz(), MODE_0);

    let shared_spi = RefCell::new(spi);

    let sdcard_spi = RefCellDevice::new(&shared_spi, DummyCsPin, timer.clone());

    // preprocess main gif
    let sdcard = embedded_sdmmc::SdCard::new(sdcard_spi, card_cs, timer.clone());
    info!("Detected sdcard size: {}", sdcard.num_bytes().unwrap());
    let mut volume_mgr = embedded_sdmmc::VolumeManager::new(sdcard, bsp::DummyTimesource);
    let mut volume0 = volume_mgr.open_volume(VolumeIdx(0)).unwrap();
    let mut root_dir = volume0.open_root_dir().unwrap();
    let mut filenames: heapless::Vec<ShortFileName, 4> = None;
    root_dir.iterate_dir(|dir_entry| {
        if dir_entry.attributes.is_hidden() || dir_entry.attributes.is_system() {
            return;
        }
        if dir_entry.name.extension() == b"GIF" {
            dir_entry.name
        }
    });

    let display_spi = RefCellDevice::new(&shared_spi, disp_cs_main, timer.clone());
    // let mut _fb = bsp::Framebuffer::new();

    let display_interface = SPIInterface::new(display_spi, disp_data_cmd);
    loop {

    }
}

