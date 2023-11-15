use bsp::Rgb565BE;
use luluu_bsp as bsp;

use embedded_sdmmc::*;

use crate::FILE_BUFFER_SIZE;

#[cfg(not(feature = "defmt"))]
use core as defmt;

pub trait ReadFile<D: BlockDevice> {
    /// Reads up to `buffer`'s length into buffer, starting at the current offset. Stores the
    /// current location in the file internally, so subsequent calls to read will start at the
    /// same place.
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Error<D::Error>>;

    /// Seeks the current offset in the file to an offset from the start of the file.
    fn seek_from_start(&mut self, offset: u32) -> Result<(), Error<D::Error>>;

    /// Seeks the current offset in the file to an offset based on the current location in the file.
    fn seek_from_current(&mut self, offset: i32) -> Result<(), Error<D::Error>>;
}

impl<'a, T, D, const MAX_DIRS: usize, const MAX_FILES: usize, const MAX_VOLUMES: usize> ReadFile<D> for File<'a, D, T, MAX_DIRS, MAX_FILES, MAX_VOLUMES>
where
    T: TimeSource,
    D: BlockDevice,
{
    #[inline(always)]
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Error<<D as BlockDevice>::Error>> {
        self.read(buffer)
    }

    #[inline(always)]
    fn seek_from_start(&mut self, offset: u32) -> Result<(), Error<<D as BlockDevice>::Error>> {
        self.seek_from_start(offset)
    }

    #[inline(always)]
    fn seek_from_current(&mut self, offset: i32) -> Result<(), Error<<D as BlockDevice>::Error>> {
        self.seek_from_current(offset)
    }
}

pub fn read_60px_frame_into_main_fb<D, F>(
    img_file: &mut F,
    file_read_buffer: &mut [u8; FILE_BUFFER_SIZE],
    fb: &mut bsp::Framebuffer<Rgb565BE, { bsp::FULL_FRAMEBUFFER_SIZE }>
)
where
    D: embedded_sdmmc::BlockDevice,
    F: ReadFile<D>,
{
    const SRC_PIXELS_SIZE: usize = 60;
    const DST_PIXELS_SIZE: usize = 240;
    const SCALE_FACTOR: usize = 4;

    // 4096 / (60 * 2) = 34 but we want evenly divisible into 60
    const SRC_ROWS_PER_BATCH: usize = 30;
    const SRC_PIXELS_PER_BATCH: usize = SRC_ROWS_PER_BATCH * SRC_PIXELS_SIZE;
    const SRC_BYTES_PER_BATCH: usize = SRC_PIXELS_PER_BATCH * 2;
    const BATCHES: usize = SRC_PIXELS_SIZE / SRC_ROWS_PER_BATCH;

    const DST_ROWS_PER_BATCH: usize = SRC_ROWS_PER_BATCH * SCALE_FACTOR;

    assert!(file_read_buffer.len() >= SRC_BYTES_PER_BATCH);
    assert!(fb.pixels().len() >= DST_PIXELS_SIZE * DST_PIXELS_SIZE);

    for batch in 0..BATCHES {
        let read_slice = &mut file_read_buffer[0..SRC_BYTES_PER_BATCH];
        let read = img_file.read(read_slice).unwrap();
        defmt::assert_eq!(read, SRC_BYTES_PER_BATCH);

        let read_pixels = unsafe { &*read_slice.as_ptr().cast::<[Rgb565BE; SRC_PIXELS_PER_BATCH]>() };

        for src_y_relative in 0..SRC_ROWS_PER_BATCH {
            let src_row_offset = (src_y_relative * SRC_PIXELS_SIZE) as isize;
            let src_row_start = unsafe { read_pixels.as_ptr().offset(src_row_offset) };
            let src_row = unsafe { &*src_row_start.cast::<[Rgb565BE; SRC_PIXELS_SIZE]>() };

            for dst_y_offset in 0..SCALE_FACTOR {
                let dst_y = (batch * DST_ROWS_PER_BATCH) + (src_y_relative * SCALE_FACTOR) + dst_y_offset;
                let dst_row_start_offset = (dst_y * DST_PIXELS_SIZE) as isize;
                let dst_row_start = unsafe { fb.pixels_mut().as_mut_ptr().offset(dst_row_start_offset) };
                let dst_row = unsafe { &mut *dst_row_start.cast::<[Rgb565BE; DST_PIXELS_SIZE]>() };
                for (dst_x, dst_pixel) in dst_row.iter_mut().enumerate() {
                    let src_x = dst_x / SCALE_FACTOR;
                    *dst_pixel = unsafe { *src_row.get_unchecked(src_x) };
                }
            }
        }
    }
}

pub fn read_120px_frame_into_main_fb<D, F>(
    img_file: &mut F,
    file_read_buffer: &mut [u8; FILE_BUFFER_SIZE],
    fb: &mut bsp::Framebuffer<Rgb565BE, { bsp::FULL_FRAMEBUFFER_SIZE }>
)
where
    D: embedded_sdmmc::BlockDevice,
    F: ReadFile<D>,
{
    const SRC_PIXELS_SIZE: usize = 120;
    const DST_PIXELS_SIZE: usize = 240;
    const SCALE_FACTOR: usize = 2;

    // 4096 / (120 * 2) = 17 but we want evenly divisible into 120
    const SRC_ROWS_PER_BATCH: usize = 15;
    const SRC_PIXELS_PER_BATCH: usize = SRC_ROWS_PER_BATCH * SRC_PIXELS_SIZE;
    const SRC_BYTES_PER_BATCH: usize = SRC_PIXELS_PER_BATCH * 2;
    const BATCHES: usize = SRC_PIXELS_SIZE / SRC_ROWS_PER_BATCH;

    const DST_ROWS_PER_BATCH: usize = SRC_ROWS_PER_BATCH * SCALE_FACTOR;

    assert!(file_read_buffer.len() >= SRC_BYTES_PER_BATCH);
    assert!(fb.pixels().len() >= DST_PIXELS_SIZE * DST_PIXELS_SIZE);

    for batch in 0..BATCHES {
        let read_slice = unsafe { &mut *file_read_buffer.as_mut_ptr().cast::<[u8; SRC_BYTES_PER_BATCH]>() };
        let read = img_file.read(read_slice).unwrap();
        defmt::debug_assert_eq!(read, SRC_BYTES_PER_BATCH);

        let read_pixels = unsafe { &*read_slice.as_ptr().cast::<[Rgb565BE; SRC_PIXELS_PER_BATCH]>() };

        for src_y_relative in 0..SRC_ROWS_PER_BATCH {
            let src_row_offset = (src_y_relative * SRC_PIXELS_SIZE) as isize;
            let src_row_start = unsafe { read_pixels.as_ptr().offset(src_row_offset) };
            let src_row = unsafe { &*src_row_start.cast::<[Rgb565BE; SRC_PIXELS_SIZE]>() };

            for dst_y_offset in 0..SCALE_FACTOR {
                let dst_y = (batch * DST_ROWS_PER_BATCH) + (src_y_relative * SCALE_FACTOR) + dst_y_offset;
                let dst_row_start_offset = (dst_y * DST_PIXELS_SIZE) as isize;
                let dst_row_start = unsafe { fb.pixels_mut().as_mut_ptr().offset(dst_row_start_offset) };
                let dst_row = unsafe { &mut *dst_row_start.cast::<[Rgb565BE; DST_PIXELS_SIZE]>() };
                for (dst_x, dst_pixel) in dst_row.iter_mut().enumerate() {
                    let src_x = dst_x / SCALE_FACTOR;
                    *dst_pixel = unsafe { *src_row.get_unchecked(src_x) };
                }
            }
        }
    }
}

pub fn read_240px_frame_into_main_fb<D, F>(
    img_file: &mut F,
    file_read_buffer: &mut [u8; FILE_BUFFER_SIZE],
    fb: &mut bsp::Framebuffer<Rgb565BE, { bsp::FULL_FRAMEBUFFER_SIZE }>
)
where
    D: embedded_sdmmc::BlockDevice,
    F: ReadFile<D>,
{
    const PIXELS_SIZE: usize = 240;

    // 4096 / (240 * 2) = 8, we want evenly divisible into 240 which it is :)
    const ROWS_PER_BATCH: usize = 8;
    const PIXELS_PER_BATCH: usize = ROWS_PER_BATCH * PIXELS_SIZE;
    const BYTES_PER_BATCH: usize = PIXELS_PER_BATCH * 2;
    const BATCHES: usize = PIXELS_SIZE / ROWS_PER_BATCH;

    assert!(file_read_buffer.len() >= BYTES_PER_BATCH);
    assert!(fb.pixels().len() >= PIXELS_SIZE * PIXELS_SIZE);

    for batch in 0..BATCHES {
        let read_slice = unsafe { &mut *file_read_buffer.as_mut_ptr().cast::<[u8; BYTES_PER_BATCH]>() };
        let read = img_file.read(read_slice).unwrap();
        defmt::debug_assert_eq!(read, BYTES_PER_BATCH);

        let read_pixels = unsafe { &*read_slice.as_ptr().cast::<[Rgb565BE; PIXELS_PER_BATCH]>() };

        let dst_y_row_offset = batch * ROWS_PER_BATCH;
        for y_relative in 0..ROWS_PER_BATCH {
            let src_row_offset = (y_relative * PIXELS_SIZE) as isize;
            let src_row_start = unsafe { read_pixels.as_ptr().offset(src_row_offset) };

            let dst_y = dst_y_row_offset + y_relative;
            let dst_row_start_offset = (dst_y * PIXELS_SIZE) as isize;
            let dst_row_start = unsafe { fb.pixels_mut().as_mut_ptr().offset(dst_row_start_offset) };
            unsafe {
                core::ptr::copy_nonoverlapping(src_row_start, dst_row_start, PIXELS_SIZE)
            }
        }
    }
}
