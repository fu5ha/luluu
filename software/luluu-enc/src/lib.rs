#![no_std]

use bytemuck::AnyBitPattern;
use bytemuck::NoUninit;
use bytemuck::TransparentWrapper;

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnyBitPattern, NoUninit, TransparentWrapper)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct Encoding(pub u8);

impl Encoding {
    pub const RGB888: Self = Self(0);
    pub const RGB565BE: Self = Self(1);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnyBitPattern, NoUninit, TransparentWrapper)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct Version(pub u8);

impl Version {
    pub const ZERO: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnyBitPattern, NoUninit, TransparentWrapper)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct MagicBytes(pub [u8; 2]);

impl MagicBytes {
    pub const CORRECT: Self = Self(*b"LU");
}

/// u16 as little-endian bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AnyBitPattern, NoUninit, TransparentWrapper)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct NumFrames(pub [u8; 2]);

impl NumFrames {
    pub fn from_u16(n: u16) -> Self {
        Self(n.to_le_bytes())
    }
    pub fn as_u16(&self) -> u16 {
        u16::from_le_bytes(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, AnyBitPattern, NoUninit)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(C, align(1))]
pub struct Header {
    pub magic: MagicBytes,
    pub version: Version,
    pub encoding: Encoding,
    pub size: u8,
    pub frame_rate: u8,
    pub n_frames: NumFrames,
}

impl Header {
    #[inline]
    pub fn decode(bytes: &[u8; HEADER_SIZE]) -> Result<Header, Error> {
        let header: Header = *bytemuck::cast_ref(bytes);
        match header.magic {
            MagicBytes::CORRECT => (),
            magic => return Err(Error::WrongMagicBytes(magic)),
        }

        match header.version {
            Version::ZERO => (),
            version => return Err(Error::UnknownVersion(version))
        }

        match header.encoding {
            Encoding::RGB565BE | Encoding::RGB888 => (),
            encoding => return Err(Error::UnknownEncoding(encoding))
        }

        Ok(header)
    }

    pub fn as_bytes(&self) -> &[u8; HEADER_SIZE] {
        bytemuck::cast_ref(self)
    }
}

pub const HEADER_SIZE: usize = core::mem::size_of::<Header>();

#[derive(Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    WrongMagicBytes(MagicBytes),
    UnknownVersion(Version),
    UnknownEncoding(Encoding),
}

#[derive(Clone, Copy, AnyBitPattern, NoUninit, TransparentWrapper)]
#[repr(transparent)]
pub struct Rgb888(pub [u8; 3]);

impl Rgb888 {
    /// View a slice of bytes that you know is RGB888 data as a slice of `Rgb888`
    #[inline(always)]
    pub fn cast_bytes(bytes: &[u8]) -> &[Self] {
        bytemuck::cast_slice(bytes)
    }

    /// View a slice of bytes that you know is RGB888 data as a slice of `Rgb888`
    #[inline(always)]
    pub fn cast_bytes_mut(bytes: &mut [u8]) -> &mut [Self] {
        bytemuck::cast_slice_mut(bytes)
    }
}
#[derive(Clone, Copy, AnyBitPattern, NoUninit, TransparentWrapper)]
#[repr(transparent)]
pub struct Rgba8888(pub [u8; 4]);

impl Rgba8888 {
    #[inline(always)]
    pub fn rgba(self) -> [u8; 4] {
        self.0
    }

    #[inline(always)]
    pub fn rgb(self) -> [u8; 3] {
        let [r, g, b, _] = self.0;
        [r, g, b]
    }

    /// View a slice of bytes that you know is RGBA8888 data as a slice of `Rgba8888`
    #[inline(always)]
    pub fn cast_bytes(bytes: &[u8]) -> &[Self] {
        bytemuck::cast_slice(bytes)
    }

    /// View a slice of bytes that you know is RGBA8888 data as a slice of `Rgba8888`
    #[inline(always)]
    pub fn cast_bytes_mut(bytes: &mut [u8]) -> &mut [Self] {
        bytemuck::cast_slice_mut(bytes)
    }
}

pub const MAX_5: u8 = 0b00011111;
pub const MAX_6: u8 = 0b00111111;

#[derive(Clone, Copy, AnyBitPattern, NoUninit, TransparentWrapper)]
#[repr(transparent)]
pub struct Rgb565NE(u16);

impl Rgb565NE {
    /// Pack an r, g, b already within the 5 bit, 6 bit and 5 bit max values into a `u16`. No
    /// clamping or etc is performed, you must make sure these values are clamped yourself.
    #[inline(always)]
    pub const fn pack_565(r_5: u8, g_6: u8, b_5: u8) -> Self {
        Self((r_5 as u16) << 11 | (g_6 as u16) << 5 | (b_5 as u16))
    }

    /// Unpack `self` to separate `r`, `g`, `b`. *does not* convert back to full 8 bit color! each
    /// component is just separated into its own u8 to work on it.
    #[inline(always)]
    pub const fn unpack_565(self) -> [u8; 3] {
        let r = ((self.0 >> 11) & 0b00011111) as u8;
        let g = ((self.0 >> 5) & 0b00111111) as u8;
        let b = (self.0 & 0b00011111) as u8;
        [r, g, b]
    }

    #[inline(always)]
    pub const fn from_raw(raw: u16) -> Self {
        Self(raw)
    }

    #[inline(always)]
    pub const fn to_raw(self) -> u16 {
        self.0
    }

    /// Convert `self` to big-endian.
    #[inline(always)]
    pub const fn to_be(self) -> Rgb565BE {
        Rgb565BE(self.0.to_be_bytes())
    }

    /// View a slice of bytes that you know is native-endian RGB565 data as a slice of `Rgb565NE`
    ///
    /// You must ensure that bytes is aligned to 2 or else this will panic.
    #[inline(always)]
    pub fn cast_bytes(bytes: &[u8]) -> &[Self] {
        bytemuck::cast_slice(bytes)
    }

    /// View a slice of bytes that you know is native-endian RGB565 data as a slice of `Rgb565NE`
    ///
    /// You must ensure that bytes is aligned to 2 or else this will panic.
    #[inline(always)]
    pub fn cast_bytes_mut(bytes: &mut [u8]) -> &mut [Self] {
        bytemuck::cast_slice_mut(bytes)
    }
}

#[derive(Clone, Copy, AnyBitPattern, NoUninit, TransparentWrapper)]
#[repr(transparent)]
pub struct Rgb565BE([u8; 2]);

impl Rgb565BE {
    pub const ZERO: Self = Self([0; 2]);

    #[inline(always)]
    pub const fn from_raw(raw: [u8; 2]) -> Self {
        Self(raw)
    }

    #[inline(always)]
    pub const fn to_raw(self) -> [u8; 2] {
        self.0
    }

    /// View a slice of bytes that you know is big-endian RGB565 data as a slice of `Rgb565BE`
    #[inline(always)]
    pub fn cast_bytes(bytes: &[u8]) -> &[Self] {
        bytemuck::cast_slice(bytes)
    }

    /// View a slice of bytes that you know is big-endian RGB565 data as a slice of `Rgb565BE`
    #[inline(always)]
    pub fn cast_bytes_mut(bytes: &mut [u8]) -> &mut [Self] {
        bytemuck::cast_slice_mut(bytes)
    }

    /// View a slice of `Rgb565BE` as a slice of bytes
    pub fn slice_as_bytes(slice: &[Self]) -> &[u8] {
        bytemuck::cast_slice(slice)
    }

    /// Convert `self` to native-endian.
    #[inline(always)]
    pub const fn to_ne(self) -> Rgb565NE {
        Rgb565NE(u16::from_be_bytes(self.0))
    }
}

