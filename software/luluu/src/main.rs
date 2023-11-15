#![no_std]
#![no_main]

use core::cell::RefCell;
use core::ops::{DerefMut, Deref};
use core::fmt::Write;

use bsp::hal::Clock;
use bsp::hal::rosc::RingOscillator;
use display_interface_spi::SPIInterface;
use embedded_hal_bus::spi::RefCellDevice;
use embedded_sdmmc::{VolumeIdx, DirEntry};
use embedded_sdmmc::sdcard::{DummyCsPin, AcquireOpts};
use luluu_bsp as bsp;

use bsp::{hal as hal, DispReset, Rgb565BE};
use bsp::{entry, hal::Spi, SpiPinLayout};
use embedded_hal::digital::{OutputPin, InputPin};

#[cfg(feature = "probe")]
use defmt_rtt as _;
#[cfg(feature = "probe")]
use panic_probe as _;
#[cfg(not(feature = "probe"))]
use panic_halt as _;
#[cfg(not(feature = "probe"))]
use core as defmt;

use bsp::hal::{
    clocks::init_clocks_and_plls,
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

use fugit::{RateExtU32, HertzU32};

use crate::read_file::read_60px_frame_into_main_fb;
use crate::read_file::read_120px_frame_into_main_fb;

mod read_file;

/// The `.sram4` section spans SRAM bank 4, which is 4KiB
pub const FILE_BUFFER_SIZE: usize = 1024 * 4;

bsp::singleton! {
    FileReadBuffer {
        #[link_section = ".sram4"]
        #[used]
        static mut FILE_BUFFER: [u8; FILE_BUFFER_SIZE] = [0u8; FILE_BUFFER_SIZE];
    }
}

bsp::singleton! {
    MainFramebuffer {
        static mut MAIN_FRAMEBUFFER: bsp::Framebuffer<Rgb565BE> = bsp::Framebuffer::const_new(Rgb565BE::ZERO);
    }
}

const SD_BAUDRATE: HertzU32 = HertzU32::kHz(31_250);
const DISP_BAUDRATE: HertzU32 = HertzU32::kHz(62_500);


#[entry]
fn main() -> ! {
    let mut peripherals = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(peripherals.WATCHDOG);

    let clocks = init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        bsp::XOSC_STABLE_DELAY_MILLIS,
        peripherals.XOSC,
        peripherals.CLOCKS,
        peripherals.PLL_SYS,
        peripherals.PLL_USB,
        &mut peripherals.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut fb = unsafe { MainFramebuffer::acquire() };
    let mut file_read_buffer = unsafe { FileReadBuffer::acquire() };

    let core = pac::CorePeripherals::take().unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, 125_000_000);

    let mut timer = hal::Timer::new(peripherals.TIMER, &mut peripherals.RESETS, &clocks);

    let sio = Sio::new(peripherals.SIO);

    let mut pins = bsp::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    let mut disp_backlight = pins.disp_backlight;
    disp_backlight.set_low().unwrap();

    pins.spi_mosi.set_slew_rate(hal::gpio::OutputSlewRate::Fast);
    pins.spi_miso.set_slew_rate(hal::gpio::OutputSlewRate::Fast);
    pins.spi_clock.set_slew_rate(hal::gpio::OutputSlewRate::Fast);

    let spi_pin_layout: SpiPinLayout = (pins.spi_mosi, pins.spi_miso, pins.spi_clock);
    let spi = Spi::new(peripherals.SPI1, spi_pin_layout);

    // start at low baud rate for initializing card
    let init = spi.init(&mut peripherals.RESETS, clocks.peripheral_clock.freq(), 200.kHz(), embedded_hal::spi::MODE_0);
    let spi = init;

    let shared_spi = RefCell::new(spi);

    let sdcard_spi: RefCellDevice<'_, Spi<hal::spi::Enabled, pac::SPI1, (hal::gpio::Pin<hal::gpio::bank0::Gpio15, hal::gpio::FunctionSpi, hal::gpio::PullNone>, hal::gpio::Pin<hal::gpio::bank0::Gpio8, hal::gpio::FunctionSpi, hal::gpio::PullDown>, hal::gpio::Pin<hal::gpio::bank0::Gpio14, hal::gpio::FunctionSpi, hal::gpio::PullNone>), 8>, DummyCsPin, hal::Timer> = RefCellDevice::new(&shared_spi, DummyCsPin, timer.clone());

    pins.card_cs.set_slew_rate(hal::gpio::OutputSlewRate::Fast);
    let card_cs = pins.card_cs;

    let sdcard = embedded_sdmmc::SdCard::new_with_options(
        sdcard_spi,
        card_cs,
        timer.clone(),
        AcquireOpts {
            use_crc: true,
            ..Default::default()
        }
    );

    let mut volume_mgr = embedded_sdmmc::VolumeManager::<_, _, 1, 1, 1>::new_with_limits(sdcard, bsp::DummyTimesource, 0);
    let mut volume0 = volume_mgr.open_volume(VolumeIdx(0)).unwrap();
    let mut root_dir = volume0.open_root_dir().unwrap();
    let mut dir_entries: heapless::Vec<DirEntry, 16> = heapless::Vec::new();
    root_dir.iterate_dir(|dir_entry| {
        if dir_entries.is_full() {
            return;
        }
        if dir_entry.attributes.is_hidden() || dir_entry.attributes.is_system() {
            return;
        }
        if dir_entry.name.extension() == b"LU" {
            dir_entries.push(dir_entry.clone()).unwrap();
        }
    }).unwrap();

    defmt::assert!(dir_entries.len() > 0);

    let mut rosc = RingOscillator::new(peripherals.ROSC).initialize();
    let file_idx = (bsp::gen_rand_u32(&mut rosc) % dir_entries.len() as u32) as usize;

    let dir_entry = &dir_entries[file_idx];

    let mut img_file = root_dir.open_file_in_dir(&dir_entry.name, embedded_sdmmc::Mode::ReadOnly).unwrap();

    let mut dir_name: heapless::String<16> = heapless::String::new();
    write!(&mut dir_name, "{}", &dir_entry.name).unwrap();
    #[cfg(feature = "probe")]
    defmt::info!("found {}, size: {}", dir_name, dir_entry.size);

    let _baud = shared_spi.borrow_mut().set_baudrate(125.MHz(), SD_BAUDRATE);
    #[cfg(feature = "probe")]
    defmt::info!("set spi baud: {}", _baud);

    let read = img_file.read(&mut file_read_buffer[0..bsp::luluu_enc::HEADER_SIZE]).unwrap();

    defmt::assert_eq!(read, bsp::luluu_enc::HEADER_SIZE);

    let mut header = bsp::luluu_enc::Header::decode((&file_read_buffer[..bsp::luluu_enc::HEADER_SIZE]).try_into().unwrap()).unwrap();

    defmt::assert_eq!(header.encoding, bsp::luluu_enc::Encoding::RGB565BE);
    header.frame_rate.make_nearest_supported();

    match header.size {
        60 => read_60px_frame_into_main_fb(&mut img_file, &mut *file_read_buffer, &mut *fb),
        120 => read_120px_frame_into_main_fb(&mut img_file, &mut *file_read_buffer, &mut *fb),
        _ => defmt::panic!("Unsupported file size: {}. Only support 120x120px or 60x60px", header.size),
    }

    #[cfg(feature = "probe")]
    defmt::info!("frame rate: {}", header.frame_rate);

    let display_frame_rate = match header.size {
        60 | 120 => {
            match header.frame_rate.0 {
                1..=2 => mipidsi::FrameRate::Hz40,
                3 => mipidsi::FrameRate::Hz42,
                4 => mipidsi::FrameRate::Hz40,
                5 => mipidsi::FrameRate::Hz40,
                6 => mipidsi::FrameRate::Hz60,
                8 => mipidsi::FrameRate::Hz72,
                10 => mipidsi::FrameRate::Hz90,
                12 => mipidsi::FrameRate::Hz72,
                15 => mipidsi::FrameRate::Hz90,
                _ => defmt::unreachable!(),
            }
        }
        240 => {
            match header.frame_rate.0 {
                1..=2 => mipidsi::FrameRate::Hz40,
                3 => mipidsi::FrameRate::Hz60,
                4..=6 => mipidsi::FrameRate::Hz90,
                8 => mipidsi::FrameRate::Hz72,
                10 => mipidsi::FrameRate::Hz90,
                12 => mipidsi::FrameRate::Hz72,
                15 => mipidsi::FrameRate::Hz90,
                _ => defmt::unreachable!(),
            }
        }
        _ => defmt::unreachable!()
    };

    let mut disp_reset = pins.disp_reset;
    disp_reset.set_slew_rate(hal::gpio::OutputSlewRate::Fast);

    disp_reset.set_low().unwrap();
    delay.delay_us(120);
    disp_reset.set_high().unwrap();
    delay.delay_ms(100);
    // disp_reset.set_low().unwrap();
    // delay.delay_us(10);
    // disp_reset.set_high().unwrap();
    // delay.delay_ms(100);

    shared_spi.borrow_mut().set_baudrate(clocks.peripheral_clock.freq(), 400.kHz());

    pins.disp_cs_main.set_slew_rate(hal::gpio::OutputSlewRate::Fast);
    pins.disp_data_cmd.set_slew_rate(hal::gpio::OutputSlewRate::Fast);
    let disp_cs_main = pins.disp_cs_main;
    let disp_data_cmd = pins.disp_data_cmd;
    let disp_vsync = pins.disp_vsync;

    let display_spi = RefCellDevice::new(&shared_spi, disp_cs_main, timer.clone());
    let display_interface = SPIInterface::new(display_spi, disp_data_cmd);
    let mut options = mipidsi::ModelOptions::with_sizes((240, 240), (240, 240));
    options.set_invert_colors(mipidsi::ColorInversion::Inverted);
    let mut display = mipidsi::Builder::new(display_interface, mipidsi::models::ST7789, options)
        .init(&mut timer, None::<DispReset>)
        .unwrap();
    display.set_tearing_effect(mipidsi::TearingEffect::Vertical).unwrap();
    display.set_frame_rate(display_frame_rate, Default::default()).unwrap();

    let _baud = shared_spi.borrow_mut().set_baudrate(clocks.peripheral_clock.freq(), DISP_BAUDRATE);
    #[cfg(feature = "probe")]
    defmt::info!("set spi baud: {}", _baud);

    let frame_budget_micros: u32 = (1_000_000 / header.frame_rate.0 as u32) - 200;

    let mut frame: u32 = 1; // because we already loaded the first frame.
    loop {
        let start_time = timer.get_counter_low();

        // we want to write starting *during* the time the controller driver is updating the lcd
        // from its internal memory, but *behind* the current place it's reading from its internal
        // memory. in this way we basically get two display-frames to update the display's memory.
        while !disp_vsync.is_high().unwrap() {}
        while disp_vsync.is_high().unwrap() {};
        delay.delay_us(300);

        // let draw_start = timer.get_counter_low();

        display.set_pixels_565be(0, 0, 240, 240, fb.as_bytes()).unwrap();
        if frame == 2 {
            disp_backlight.set_high().unwrap();
        }

        // let draw_end = timer.get_counter_low();
        // if frame % 32 == 0 {
        //     defmt::info!("draw took: {}us", draw_end - draw_start);
        // }

        shared_spi.borrow_mut().set_baudrate(clocks.peripheral_clock.freq(), SD_BAUDRATE);

        // let modify_start = timer.get_counter_low();

        if frame % header.n_frames.as_u16() as u32 == 0 {
            img_file.seek_from_start(bsp::luluu_enc::HEADER_SIZE as u32).unwrap();
        }

        match header.size {
            60 => read_60px_frame_into_main_fb(&mut img_file, &mut *file_read_buffer, &mut *fb),
            120 => read_120px_frame_into_main_fb(&mut img_file, &mut *file_read_buffer, &mut *fb),
            _ => defmt::unreachable!(),
        }

        // let frame_mult = 0.5 + (0.5 * (frame as f32 / 2.0).cos());
        // let frame_mult_5bit_as_u16 = (frame_mult * Rgb565::MAX_R as f32) as u16;
        // let frame_mult_6bit_as_u16 = (frame_mult * Rgb565::MAX_G as f32) as u16;
        //
        // for (fb_pixel, pixel) in fb.pixels_mut().iter_mut().zip(orig_img_fb.pixels()) {
        //     let (r, g, b) = pixel.unpack_565();
        //     let r = ((r as u16 * frame_mult_5bit_as_u16) >> 5) as u8;
        //     let g = ((g as u16 * frame_mult_6bit_as_u16) >> 6) as u8;
        //     let b = ((b as u16 * frame_mult_5bit_as_u16) >> 5) as u8;
        //     let pixel_565 = Rgb565NE::pack_565(r, g, b);
        //     *fb_pixel = pixel_565.to_be();
        // }
        //
        // let modify_end = timer.get_counter_low();
        // if (frame + 2) % 32 == 0 {
        //     defmt::info!("modify took: {}us", modify_end - modify_start);
        // }

        let _= shared_spi.borrow_mut().set_baudrate(clocks.peripheral_clock.freq(), DISP_BAUDRATE);

        let current_time = timer.get_counter_low();
        let frame_time = current_time - start_time;
        if let Some(micros_left) = frame_budget_micros.checked_sub(frame_time) {
            // if (frame + 4) % 32 == 0 {
            //     defmt::info!("waiting for frame: {}us", micros_left);
            // }
            delay.delay_us(micros_left);
        } else {
            // if (frame + 4) % 32 == 0 {
            //     defmt::info!("frame overbudget, took: {}us", frame_time);
            // }
        }

        frame += 1;
    }
}
