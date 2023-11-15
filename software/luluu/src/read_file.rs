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

    // 4096 / (120 * 2) = 17 but we want evenly divisible into 60
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

// #[allow(dead_code)]
// pub fn read_sd_frame_into_intermediate_fb<D, F>(
//     img_file: &mut F,
//     header: &bsp::luluu_enc::Header,
//     file_read_buffer: &mut heapless::Vec<u8, { FILE_BUFFER_SIZE }>,
//     fb: &mut bsp::Framebuffer<Rgb565BE, { bsp::HALF_FRAMEBUFFER_SIZE }>
// ) -> Result<(), WrongSize>
// where
//     D: embedded_sdmmc::BlockDevice,
//     F: ReadFile<D>,
// {
//     let side_size: usize = match header.size {
//         60 => 60,
//         120 => 120,
//         _ => return Err(WrongSize),
//     };
//     let total_pixels_per_frame = side_size * side_size;
//     let data_bytes_per_frame = total_pixels_per_frame * 2;
//     let mut i = 0;
//     let mut bytes_read = 0;
//     while i < total_pixels_per_frame {
//         let to_read_this_batch = (data_bytes_per_frame - bytes_read).min(FILE_BUFFER_SIZE);

//         file_read_buffer.clear();
//         file_read_buffer.resize_default(to_read_this_batch).unwrap();

//         let read = img_file.read(file_read_buffer).unwrap();
//         defmt::debug_assert_eq!(read, to_read_this_batch);
//         bytes_read += to_read_this_batch;

//         let read_pixels = Rgb565BE::cast_bytes(&*file_read_buffer);
//         let dst_pixels = &mut fb.pixels_mut()[i..(i + read_pixels.len())];
//         dst_pixels.copy_from_slice(read_pixels);

//         i += 1;
//     }

//     Ok(())
// }

// #[allow(dead_code)]
// pub fn blit_to_main_fb<const SCALE: usize>(
//     intermediate: &bsp::Framebuffer<Rgb565BE, { bsp::HALF_FRAMEBUFFER_SIZE }>,
//     main: &mut bsp::Framebuffer<Rgb565BE>,
// ) {
//     let (shr, src_side_len) = match SCALE {
//         2 => (1, 120),
//         4 => (2, 60),
//         _ => defmt::unreachable!("Bad scaling factor for blit"),
//     };

//     for (y, row) in main.pixels_mut().chunks_exact_mut(240).enumerate() {
//         let src_y = y >> shr;
//         for (x, dst_pixel) in row.iter_mut().enumerate() {
//             let src_x = x >> shr;
//             let src_i = src_x + src_y * src_side_len;
//             *dst_pixel = *unsafe { intermediate.pixels().get_unchecked(src_i) };
//         }
//     }
// }
