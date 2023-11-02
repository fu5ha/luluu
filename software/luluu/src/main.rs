#![no_std]
#![no_main]

use core::cell::RefCell;

use display_interface_spi::SPIInterface;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{RgbColor, IntoStorage, ImageDrawable, DrawTarget};
use embedded_hal::delay::DelayUs;
use embedded_hal_bus::spi::RefCellDevice;
// use embedded_sdmmc::{VolumeIdx, ShortFileName};
use luluu_bsp as bsp;

use bsp::hal as hal;
use bsp::{entry, hal::Spi, SpiPinLayout};
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::OutputPin;
use panic_probe as _;
// use panic_halt as _;

use bsp::hal::{
    clocks::init_clocks_and_plls,
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

use fugit::RateExtU32;

#[entry]
fn main() -> ! {
    // info!("Program start");
    let mut peripherals = pac::Peripherals::take().unwrap();
    // let core = pac::CorePeripherals::take().unwrap();
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
    backlight_pin.set_low().unwrap();
    backlight_pin.set_high().unwrap();

    // let card_cs = pins.card_cs;

    let disp_cs_main = pins.disp_cs_main;
    let disp_data_cmd = pins.disp_data_cmd;
    let disp_reset = pins.disp_reset;

    let spi_pin_layout: SpiPinLayout = (pins.spi_mosi, pins.spi_miso, pins.spi_clock);
    let spi = Spi::new(peripherals.SPI1, spi_pin_layout);

    // start at low baud rate for initializing card
    let init = spi.init(&mut peripherals.RESETS, 125.MHz(), 400.kHz(), embedded_hal::spi::MODE_2);
    let spi = init;

    let shared_spi = RefCell::new(spi);

    // let sdcard_spi = RefCellDevice::new(&shared_spi, DummyCsPin, timer.clone());

    // // preprocess main gif
    // let sdcard = embedded_sdmmc::SdCard::new(sdcard_spi, card_cs, timer.clone());
    // info!("Detected sdcard size: {}", sdcard.num_bytes().unwrap());
    // let mut volume_mgr = embedded_sdmmc::VolumeManager::new(sdcard, bsp::DummyTimesource);
    // let mut volume0 = volume_mgr.open_volume(VolumeIdx(0)).unwrap();
    // let mut root_dir = volume0.open_root_dir().unwrap();
    // let mut filenames: heapless::Vec<ShortFileName, 4> = heapless::Vec::new();
    // root_dir.iterate_dir(|dir_entry| {
    //     if filenames.is_full() {
    //         return;
    //     }
    //     if dir_entry.attributes.is_hidden() || dir_entry.attributes.is_system() {
    //         return;
    //     }
    //     if dir_entry.name.extension() == b"GIF" {
    //         filenames.push(dir_entry.name.clone()).unwrap();
    //     }
    // }).unwrap();

    shared_spi.borrow_mut().set_baudrate(125.MHz(), 64.MHz());

    let display_spi = RefCellDevice::new(&shared_spi, disp_cs_main, timer.clone());
    let display_interface = SPIInterface::new(display_spi, disp_data_cmd);
    let mut options = mipidsi::ModelOptions::with_sizes((240, 240), (240, 240));
    options.set_invert_colors(mipidsi::ColorInversion::Inverted);
    let mut display = mipidsi::Builder::new(display_interface, mipidsi::models::ST7789, options)
        .init(&mut timer, Some(disp_reset))
        .unwrap();
    display.set_tearing_effect(mipidsi::TearingEffect::Vertical).unwrap();
    display.set_frame_rate(mipidsi::FrameRate::Hz40, Default::default()).unwrap();

    let mut fb = bsp::Framebuffer::new();

    const FRAME_RATE: u32 = 24;
    const FRAME_BUDGET_MICROS: u32 = 1_000_000 / FRAME_RATE;

    let mut frame: usize = 0;
    loop {
        let start_time = timer.get_counter_low();
        let pixels: &mut [u16] = bytemuck::cast_slice_mut(fb.data_mut());
        for (i, raw_pixel) in pixels.iter_mut().enumerate() {
            let x = i % 240;
            let y = i / 240;

            let x = x / 3;
            let y = y / 3;

            let offset = frame / 4;
            let scaled_i = x + (y * 240 / 3);

            let out_color = match (scaled_i + offset) % 3 {
                0 => Rgb565::RED,
                1 => Rgb565::GREEN,
                2 => Rgb565::BLUE,
                _ => crate::unreachable!(),
            };
            // checkerboard
            // let is_white = if y & 1 == 0 {
            //     if x & 1 == 0 {
            //         false
            //     } else {
            //         true
            //     }
            // } else {
            //     if x & 1 == 0 {
            //         true
            //     } else {
            //         false
            //     }
            // };
            // let is_white = if (frame / 4) & 1 == 0 { !is_white } else { is_white };
            // let out_color: Rgb565 = if is_white {
            //     Rgb565::BLUE
            // } else {
            //     Rgb565::GREEN
            // };
            *raw_pixel = out_color.into_storage();
        }
        // display.set_pixels(0, 0, 240, 240, &*pixels);
        fb.as_image().draw(&mut display).unwrap();
        // display.clear(Rgb565::RED).unwrap();

        let current_time = timer.get_counter_low();
        if let Some(micros_left) = FRAME_BUDGET_MICROS.checked_sub(current_time - start_time) {
            timer.delay_us(micros_left);
        }

        frame += 1;
    }
}

