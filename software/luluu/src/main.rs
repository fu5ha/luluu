#![no_std]
#![no_main]

use core::cell::RefCell;
use core::ops::{DerefMut, Deref};
use core::fmt::Write;

use bsp::hal::Clock;
use bsp::hal::rosc::RingOscillator;
use display_interface_spi::SPIInterface;
use embedded_hal_bus::spi::RefCellDevice;
use embedded_sdmmc::{VolumeIdx, DirEntry, BlockDevice};
use embedded_sdmmc::sdcard::{DummyCsPin, AcquireOpts};
use luluu_bsp as bsp;

use bsp::{hal as hal, DispReset, Rgb565BE};
use bsp::{entry, hal::Spi, SpiPinLayout};
use defmt_rtt as _;
use embedded_hal::digital::{OutputPin, InputPin};
use panic_probe as _;
// use panic_halt as _;

use bsp::hal::{
    clocks::init_clocks_and_plls,
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

use fugit::{RateExtU32, HertzU32};

const FILE_BUFFER_SIZE: usize = 1024 * 16;

bsp::singleton! {
    FileReadBuffer {
        // #[link_section = ".sram"]
        // #[used]
        static mut FILE_BUFFER: heapless::Vec<u8, FILE_BUFFER_SIZE> = heapless::Vec::new();
    }
}

bsp::singleton! {
    MainFramebuffer {
        static mut MAIN_FRAMEBUFFER: bsp::Framebuffer<Rgb565BE> = bsp::Framebuffer::const_new(Rgb565BE::ZERO);
    }
}

pub trait SdFile<D: embedded_sdmmc::BlockDevice> {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, embedded_sdmmc::Error<D::Error>>;

    fn seek_from_start(&mut self, offset: u32) -> Result<(), embedded_sdmmc::Error<D::Error>>;
}

impl<'a, T, D, const MAX_DIRS: usize, const MAX_FILES: usize, const MAX_VOLUMES: usize> SdFile<D> for embedded_sdmmc::File<'a, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>
where
    T: embedded_sdmmc::TimeSource,
    D: embedded_sdmmc::BlockDevice,
{
    #[inline(always)]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, embedded_sdmmc::Error<<D as BlockDevice>::Error>> {
        self.read(buffer)
    }

    #[inline(always)]
    fn seek_from_start(&mut self, offset: u32) -> Result<(), embedded_sdmmc::Error<<D as BlockDevice>::Error>> {
        self.seek_from_start(offset)
    }
}

pub fn read_sd_frame_into_framebuffer<D, F>(img_file: &mut F, size: usize, scale_factor: usize, file_read_buffer: &mut heapless::Vec<u8, FILE_BUFFER_SIZE>, fb: &mut bsp::Framebuffer<Rgb565BE>)
where
    D: embedded_sdmmc::BlockDevice,
    F: SdFile<D>,
{
    let total_pixels_per_frame = size * size;
    let data_bytes_per_frame = total_pixels_per_frame * 2;
    let mut i = 0;
    let mut bytes_read = 0;
    while i < total_pixels_per_frame {
        let to_read_this_batch = (data_bytes_per_frame - bytes_read).min(FILE_BUFFER_SIZE);

        file_read_buffer.clear();
        file_read_buffer.resize_default(to_read_this_batch).unwrap();

        let read = img_file.read(file_read_buffer).unwrap();
        defmt::debug_assert_eq!(read, to_read_this_batch);
        bytes_read += to_read_this_batch;

        let read_pixels = Rgb565BE::cast_bytes(&*file_read_buffer);
        for read_pixel in read_pixels {
            let x = i % size;
            let y = i / size;

            let x_to_write_start_inclu = x * scale_factor;
            let x_to_write_end_exclu = x_to_write_start_inclu + scale_factor;

            let y_to_write_start_inclu = y * scale_factor;
            let y_to_write_end_exclu = y_to_write_start_inclu + scale_factor;


            for x in x_to_write_start_inclu..x_to_write_end_exclu {
                for y in y_to_write_start_inclu..y_to_write_end_exclu {
                    let i = x + y * 240;
                    unsafe { *fb.pixels_mut().get_unchecked_mut(i) = *read_pixel; }
                }
            }

            i += 1;
        }
    }
}

const SD_BAUDRATE: HertzU32 = HertzU32::kHz(31_250);
const DISP_BAUDRATE: HertzU32 = HertzU32::kHz(62_500);

#[entry]
fn main() -> ! {
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

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let mut timer = hal::Timer::new(peripherals.TIMER, &mut peripherals.RESETS, &clocks);

    let mut pins = bsp::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    pins.set_fast_slew();

    let mut disp_backlight = pins.disp_backlight;
    let disp_cs_main = pins.disp_cs_main;
    let disp_data_cmd = pins.disp_data_cmd;
    let mut disp_reset = pins.disp_reset;
    let disp_vsync = pins.disp_vsync;

    disp_backlight.set_low().unwrap();
    disp_reset.set_low().unwrap();

    let mut fb = unsafe { MainFramebuffer::acquire() };
    let mut file_read_buffer = unsafe { FileReadBuffer::acquire() };

    let spi_pin_layout: SpiPinLayout = (pins.spi_mosi, pins.spi_miso, pins.spi_clock);
    let spi = Spi::new(peripherals.SPI1, spi_pin_layout);

    // start at low baud rate for initializing card
    let init = spi.init(&mut peripherals.RESETS, clocks.peripheral_clock.freq(), 200.kHz(), embedded_hal::spi::MODE_0);
    let spi = init;

    let shared_spi = RefCell::new(spi);

    let sdcard_spi = RefCellDevice::new(&shared_spi, DummyCsPin, timer.clone());

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

    let mut volume_mgr = embedded_sdmmc::VolumeManager::new(sdcard, bsp::DummyTimesource);
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
    defmt::info!("found {}, size: {}", dir_name, dir_entry.size);

    file_read_buffer.resize_default(bsp::luluu_enc::HEADER_SIZE).unwrap();

    let baud = shared_spi.borrow_mut().set_baudrate(125.MHz(), SD_BAUDRATE);
    defmt::info!("set spi baud: {}", baud);

    let read = img_file.read(&mut file_read_buffer).unwrap();

    defmt::assert_eq!(read, bsp::luluu_enc::HEADER_SIZE);

    let header = bsp::luluu_enc::Header::decode(file_read_buffer.as_slice().try_into().unwrap()).unwrap();

    defmt::assert_eq!(header.encoding, bsp::luluu_enc::Encoding::RGB565BE);

    let size = header.size as usize;

    let scale_factor = match size {
        60 => 4usize,
        120 => 2usize,
        240 => 1usize,
        size => defmt::panic!("bad image size: {}", size),
    };

    read_sd_frame_into_framebuffer(&mut img_file, size, scale_factor, &mut *file_read_buffer, &mut *fb);

    // let bluenoise_data = include_bytes!("../data/bluenoise32.bmp");

    let _ = shared_spi.borrow_mut().set_baudrate(clocks.peripheral_clock.freq(), 400.kHz());

    disp_reset.set_high().unwrap();

    let display_spi = RefCellDevice::new(&shared_spi, disp_cs_main, timer.clone());
    let display_interface = SPIInterface::new(display_spi, disp_data_cmd);
    let mut options = mipidsi::ModelOptions::with_sizes((240, 240), (240, 240));
    options.set_invert_colors(mipidsi::ColorInversion::Inverted);
    let mut display = mipidsi::Builder::new(display_interface, mipidsi::models::ST7789, options)
        .init(&mut timer, None::<DispReset>)
        .unwrap();
    display.set_tearing_effect(mipidsi::TearingEffect::Vertical).unwrap();
    display.set_frame_rate(mipidsi::FrameRate::Hz40, Default::default()).unwrap();

    let baud = shared_spi.borrow_mut().set_baudrate(clocks.peripheral_clock.freq(), DISP_BAUDRATE);
    defmt::info!("set spi baud: {}", baud);

    display.set_pixels_565be(0, 0, 240, 240, fb.as_bytes()).unwrap();

    disp_backlight.set_high().unwrap();

    let frame_budget_micros: u32 = (1_000_000 / header.frame_rate as u32) - 200;

    let mut frame: u32 = 1; // because we already loaded the first frame.
    loop {
        let start_time = timer.get_counter_low();

        // we want to write staring *during* the time the controller driver is updating the lcd
        // from its internal memory, but *behind* the current place it's reading from its internal
        // memory. in this way we basically get two display-frames to update the display's memory.
        while !disp_vsync.is_high().unwrap() {}
        while disp_vsync.is_high().unwrap() {};
        delay.delay_us(500);

        let draw_start = timer.get_counter_low();

        display.set_pixels_565be(0, 0, 240, 240, fb.as_bytes()).unwrap();

        let draw_end = timer.get_counter_low();
        if frame % 32 == 0 {
            defmt::info!("draw took: {}us", draw_end - draw_start);
        }

        // let frame_mult = 0.5 + (0.5 * (frame as f32 / 2.0).cos());
        // let frame_mult_5bit_as_u16 = (frame_mult * Rgb565::MAX_R as f32) as u16;
        // let frame_mult_6bit_as_u16 = (frame_mult * Rgb565::MAX_G as f32) as u16;

        let _ = shared_spi.borrow_mut().set_baudrate(clocks.peripheral_clock.freq(), SD_BAUDRATE);

        let modify_start = timer.get_counter_low();

        if frame % header.n_frames.as_u16() as u32 == 0 {
            img_file.seek_from_start(bsp::luluu_enc::HEADER_SIZE as u32).unwrap();
        }

        read_sd_frame_into_framebuffer(&mut img_file, size, scale_factor, &mut *file_read_buffer, &mut *fb);

        // for (fb_pixel, pixel) in fb.pixels_mut().iter_mut().zip(orig_img_fb.pixels()) {
        //     let (r, g, b) = pixel.unpack_565();
        //     let r = ((r as u16 * frame_mult_5bit_as_u16) >> 5) as u8;
        //     let g = ((g as u16 * frame_mult_6bit_as_u16) >> 6) as u8;
        //     let b = ((b as u16 * frame_mult_5bit_as_u16) >> 5) as u8;
        //     let pixel_565 = Rgb565NE::pack_565(r, g, b);
        //     *fb_pixel = pixel_565.to_be();
        // }

        let modify_end = timer.get_counter_low();
        if (frame + 2) % 32 == 0 {
            defmt::info!("modify took: {}us", modify_end - modify_start);
        }

        let _= shared_spi.borrow_mut().set_baudrate(clocks.peripheral_clock.freq(), DISP_BAUDRATE);

        // for (i, raw_pixel) in pixels.iter_mut().enumerate() {

        //     let x = i % 240;
        //     let y = i / 240;

        //     let x = x / 3;
        //     let y = y / 3;

        //     let offset = frame / 4;
        //     let scaled_i = x + (y * 240 / 3);

        //     let out_color = match (scaled_i + offset) % 3 {
        //         0 => Rgb565::RED,
        //         1 => Rgb565::GREEN,
        //         2 => Rgb565::BLUE,
        //         _ => crate::unreachable!(),
        //     };
        //     checkerboard
        //     let is_white = if y & 1 == 0 {
        //         if x & 1 == 0 {
        //             false
        //         } else {
        //             true
        //         }
        //     } else {
        //         if x & 1 == 0 {
        //             true
        //         } else {
        //             false
        //         }
        //     };
        //     let is_white = if (frame / 4) & 1 == 0 { !is_white } else { is_white };
        //     let out_color: Rgb565 = if is_white {
        //         Rgb565::BLUE
        //     } else {
        //         Rgb565::GREEN
        //     };
        //     *raw_pixel = out_color.into_storage();
        // }
        // display.set_pixels(0, 0, 240, 240, &*pixels);
        // fb.as_image().draw(&mut display).unwrap();
        // display.clear(Rgb565::RED).unwrap();

        let current_time = timer.get_counter_low();
        let frame_time = current_time - start_time;
        if let Some(micros_left) = frame_budget_micros.checked_sub(frame_time) {
            if (frame + 4) % 32 == 0 {
                defmt::info!("waiting for frame: {}us", micros_left);
            }
            delay.delay_us(micros_left);
        } else {
            if (frame + 4) % 32 == 0 {
                defmt::info!("frame overbudget, took: {}us", frame_time);
            }
        }

        frame += 1;
    }
}
