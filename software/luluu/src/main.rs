#![no_std]
#![no_main]

use bsp::hal::i2c::peripheral;
use display_interface_spi::SPIInterface;
use luluu_bsp as bsp;

use bsp::hal as hal;
use bsp::{entry, hal::Spi, SpiPinLayout, DispBacklightToggle, Framebuffer};
use defmt::*;
use defmt_rtt as _;
use embedded_hal::{digital::v2::OutputPin, spi::MODE_0};
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

    let mut timer = hal::Timer::new(peripherals.TIMER, &mut peripherals.RESETS, &clocks);

    let pins = bsp::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    let mut backlight_pin = pins.disp_backlight;
    backlight_pin.set_high();

    let card_cs = pins.card_cs;

    let disp_cs_main = pins.disp_cs_main;
    let disp_data_cmd = pins.disp_data_cmd;


    let spi_pin_layout: SpiPinLayout = (pins.spi_mosi, pins.spi_miso, pins.spi_clock);
    let spi = Spi::<_, _, _, 8>::new(peripherals.SPI1, spi_pin_layout);

    // start at low baud rate for initializing card
    let spi = spi.init(&mut peripherals.RESETS, 125.MHz(), 400.kHz(), MODE_0);

    let mut fb = bsp::Framebuffer::new();

    // TODO: add bootup/load animation

    // preprocess main gif
    let sdcard = embedded_sdmmc::SdCard::new(spi, card_cs, timer.clone());
    let mut volume_mgr = embedded_sdmmc::VolumeManager::new(sdcard, bsp::DummyTimesource);



    loop {
        let sdcard = embedded_sdmmc::SdCard::new(spi, card_cs, timer.clone());

        let display_interface = SPIInterface::new(spi, disp_data_cmd, disp_cs_main);

    }
}

