use core::convert::TryFrom;

/// A trait for parsing raw binary data.
///
/// This is a low-level, internal trait that should not be used directly.
pub trait FromData: Sized {
    /// Stores an object size in raw data.
    ///
    /// `mem::size_of` by default.
    ///
    /// Override when size of `Self` != size of a raw data.
    /// For example, when you are parsing `u16`, but storing it as `u8`.
    /// In this case `size_of::<Self>()` == 1, but `FromData::SIZE` == 2.
    const SIZE: usize = core::mem::size_of::<Self>();

    /// Parses an object from a raw data.
    ///
    /// This method **must** not panic and **must** not read past the bounds.
    fn parse(data: &[u8]) -> Self;
}

impl FromData for u8 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        data[0]
    }
}

impl FromData for i8 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        data[0] as i8
    }
}

impl FromData for u16 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        u16::from_be_bytes([data[0], data[1]])
    }
}

impl FromData for i16 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        i16::from_be_bytes([data[0], data[1]])
    }
}

impl FromData for u32 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        // For u32 it's faster to use TryInto, but for u16/i16 it's faster to index.
        use core::convert::TryInto;
        u32::from_be_bytes(data.try_into().unwrap())
    }
}

impl FromData for i32 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        // For i32 it's faster to use TryInto, but for u16/i16 it's faster to index.
        use core::convert::TryInto;
        i32::from_be_bytes(data.try_into().unwrap())
    }
}


// https://docs.microsoft.com/en-us/typography/opentype/spec/otff#data-types
#[derive(Clone, Copy, Debug)]
pub struct U24(pub u32);

impl FromData for U24 {
    const SIZE: usize = 3;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        U24(u32::from(data[0]) << 16 | u32::from(data[1]) << 8 | u32::from(data[2]))
    }
}


/// A 16-bit signed fixed number with the low 14 bits of fraction (2.14).
#[derive(Clone, Copy, Debug)]
pub struct F2DOT14(pub i16);

impl F2DOT14 {
    #[inline]
    pub fn to_float(&self) -> f32 {
        f32::from(self.0) / 16384.0
    }
}

impl FromData for F2DOT14 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        F2DOT14(i16::parse(data))
    }
}


/// A 32-bit signed fixed-point number (16.16).
#[derive(Clone, Copy, Debug)]
pub struct Fixed(pub f32);

impl FromData for Fixed {
    const SIZE: usize = 4;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        // TODO: is it safe to cast?
        Fixed(i32::parse(data) as f32 / 65536.0)
    }
}


pub trait NumConv<T>: Sized {
    fn num_from(_: T) -> Self;
}

// Rust doesn't implement `From<u32> for usize`,
// because it has to support 16 bit targets.
// We don't, so we can allow this.
impl NumConv<u32> for usize {
    #[inline]
    fn num_from(v: u32) -> Self {
        debug_assert!(core::mem::size_of::<usize>() >= 4);
        v as usize
    }
}


/// Just like TryFrom<N>, but for numeric types not supported by the Rust's std.
pub trait TryNumConv<T>: Sized {
    fn try_num_from(_: T) -> Option<Self>;
}

impl TryNumConv<f32> for i16 {
    #[inline]
    fn try_num_from(v: f32) -> Option<Self> {
        i32::try_num_from(v).and_then(|v| i16::try_from(v).ok())
    }
}

impl TryNumConv<f32> for u16 {
    #[inline]
    fn try_num_from(v: f32) -> Option<Self> {
        i32::try_num_from(v).and_then(|v| u16::try_from(v).ok())
    }
}

impl TryNumConv<f32> for i32 {
    #[inline]
    fn try_num_from(v: f32) -> Option<Self> {
        // Based on https://github.com/rust-num/num-traits/blob/master/src/cast.rs

        // We can't represent `MIN-1` exactly, but there's no fractional part
        // at this magnitude, so we can just use a `MIN` inclusive boundary.
        const MIN: f32 = core::i32::MIN as f32;
        // We can't represent `MAX` exactly, but it will round up to exactly
        // `MAX+1` (a power of two) when we cast it.
        const MAX_P1: f32 = core::i32::MAX as f32;
        if v >= MIN && v < MAX_P1 {
            Some(v as i32)
        } else {
            None
        }
    }
}


/// A u16/u32 length type used by `LazyArray`.
pub trait ArraySize
    : core::ops::Add<Output=Self>
    + core::ops::AddAssign
    + core::ops::Sub<Output=Self>
    + core::ops::SubAssign
    + core::ops::Div<Output=Self>
    + PartialOrd
    + Sized
    + Copy
{
    /// Associated 0.
    const ZERO: Self;
    /// Associated 1.
    const ONE: Self;
    /// Associated 2.
    const TWO: Self;

    /// Creates `ArraySize` from `usize`;
    fn from_usize(n: usize) -> Self;

    /// Converts `ArraySize` to `usize`.
    fn to_usize(&self) -> usize;
}

impl ArraySize for u16 {
    const ZERO: Self = 0;
    const ONE: Self = 1;
    const TWO: Self = 2;

    #[inline]
    fn from_usize(n: usize) -> Self {
        debug_assert!(n <= core::u16::MAX as usize);
        n as u16
    }

    #[inline]
    fn to_usize(&self) -> usize { usize::from(*self) }
}

impl ArraySize for u32 {
    const ZERO: Self = 0;
    const ONE: Self = 1;
    const TWO: Self = 2;

    #[inline]
    fn from_usize(n: usize) -> Self {
        debug_assert!(n <= core::u32::MAX as usize);
        n as u32
    }

    #[inline]
    fn to_usize(&self) -> usize { usize::num_from(*self) }
}


/// A slice-like container that converts internal binary data only on access.
///
/// This is a low-level, internal structure that should not be used directly.
#[derive(Clone, Copy)]
pub struct LazyArray<'a, T, Idx> {
    data: &'a [u8],
    data_type: core::marker::PhantomData<T>,
    len_type: core::marker::PhantomData<Idx>,
}

impl<T, Idx> Default for LazyArray<'_, T, Idx> {
    fn default() -> Self {
        LazyArray {
            data: &[],
            data_type: core::marker::PhantomData,
            len_type: core::marker::PhantomData,
        }
    }
}

impl<'a, T: FromData, Idx: ArraySize> LazyArray<'a, T, Idx> {
    /// Creates a new `LazyArray`.
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        LazyArray {
            data,
            data_type: core::marker::PhantomData,
            len_type: core::marker::PhantomData,
        }
    }

    pub(crate) fn at(&self, index: Idx) -> T {
        let start = index.to_usize() * T::SIZE;
        let end = start + T::SIZE;
        T::parse(&self.data[start..end])
    }

    /// Returns a value at `index`.
    pub fn get(&self, index: Idx) -> Option<T> {
        if index < self.len() {
            let start = index.to_usize() * T::SIZE;
            let end = start + T::SIZE;
            Some(T::parse(&self.data[start..end]))
        } else {
            None
        }
    }

    /// Returns the last value.
    #[inline]
    pub fn last(&self) -> Option<T> {
        if !self.is_empty() {
            self.get(self.len() - Idx::ONE)
        } else {
            None
        }
    }

    /// Returns array's length.
    #[inline]
    pub fn len(&self) -> Idx {
        Idx::from_usize(self.data.len() / T::SIZE)
    }

    /// Checks if array is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == Idx::ZERO
    }

    /// Performs a binary search by specified `key`.
    #[inline]
    pub fn binary_search(&self, key: &T) -> Option<(Idx, T)>
        where T: Ord
    {
        self.binary_search_by(|p| p.cmp(key))
    }

    /// Performs a binary search using specified closure.
    #[inline]
    pub fn binary_search_by<F>(&self, mut f: F) -> Option<(Idx, T)>
        where F: FnMut(&T) -> core::cmp::Ordering
    {
        // Based on Rust std implementation.

        use core::cmp::Ordering;

        let mut size = self.len();
        if size == Idx::ZERO {
            return None;
        }

        let mut base = Idx::ZERO;
        while size > Idx::ONE {
            let half = size / Idx::TWO;
            let mid = base + half;
            // mid is always in [0, size), that means mid is >= 0 and < size.
            // mid >= 0: by definition
            // mid < size: mid = size / 2 + size / 4 + size / 8 ...
            let cmp = f(&self.at(mid));
            base = if cmp == Ordering::Greater { base } else { mid };
            size -= half;
        }

        // base is always in [0, size) because base <= mid.
        let value = self.at(base);
        if f(&value) == Ordering::Equal { Some((base, value)) } else { None }
    }
}

impl<'a, T: FromData + core::fmt::Debug + Copy, Idx: ArraySize> core::fmt::Debug for LazyArray<'a, T, Idx> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_list().entries(self.into_iter()).finish()
    }
}

impl<'a, T: FromData, Idx: ArraySize> IntoIterator for LazyArray<'a, T, Idx> {
    type Item = T;
    type IntoIter = LazyArrayIter<'a, T, Idx>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        LazyArrayIter {
            data: self,
            index: Idx::ZERO,
        }
    }
}

/// An alias to `LazyArray` with max length equal to `u16`.
pub type LazyArray16<'a, T> = LazyArray<'a, T, u16>;

/// An iterator over `LazyArray`.
#[derive(Clone, Copy)]
pub struct LazyArrayIter<'a, T, Idx: ArraySize> {
    data: LazyArray<'a, T, Idx>,
    index: Idx,
}

impl<T, Idx: ArraySize> Default for LazyArrayIter<'_, T, Idx> {
    fn default() -> Self {
        LazyArrayIter {
            data: LazyArray::default(),
            index: Idx::ZERO,
        }
    }
}

impl<'a, T: FromData, Idx: ArraySize> Iterator for LazyArrayIter<'a, T, Idx> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.index += Idx::ONE; // TODO: check
        self.data.get(self.index - Idx::ONE)
    }

    #[inline]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.data.get(ArraySize::from_usize(n))
    }
}

impl<T: FromData, Idx: ArraySize> core::fmt::Debug for LazyArrayIter<'_, T, Idx> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "LazyArrayIter()")
    }
}


#[derive(Clone, Copy, Default)]
pub struct Stream<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Stream<'a> {
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        Stream {
            data,
            offset: 0,
        }
    }

    #[inline]
    pub fn new_at(data: &'a [u8], offset: usize) -> Self {
        Stream {
            data,
            offset,
        }
    }

    #[inline]
    pub fn at_end(&self) -> bool {
        self.offset >= self.data.len()
    }

    #[inline]
    pub fn jump_to_end(&mut self) {
        self.offset = self.data.len();
    }

    #[inline]
    pub fn offset(&self) -> usize {
        self.offset
    }

    #[inline]
    pub fn tail(&self) -> Option<&'a [u8]> {
        self.data.get(self.offset..self.data.len())
    }

    #[inline]
    pub fn skip<T: FromData>(&mut self) {
        self.offset += T::SIZE;
    }

    #[inline]
    pub fn advance<L: ArraySize>(&mut self, len: L) {
        self.offset += len.to_usize();
    }

    #[inline]
    pub fn read<T: FromData>(&mut self) -> Option<T> {
        let start = self.offset;
        self.offset += T::SIZE;
        let end = self.offset;

        let data = self.data.get(start..end)?;
        Some(T::parse(data))
    }

    #[inline]
    pub fn read_at<T: FromData>(data: &[u8], mut offset: usize) -> Option<T> {
        let start = offset;
        offset += T::SIZE;
        let end = offset;

        let data = data.get(start..end)?;
        Some(T::parse(data))
    }

    #[inline]
    pub fn read_bytes<L: ArraySize>(&mut self, len: L) -> Option<&'a [u8]> {
        let offset = self.offset;
        self.offset += len.to_usize();
        self.data.get(offset..(offset + len.to_usize()))
    }

    #[inline]
    pub fn read_array<T: FromData, Idx: ArraySize>(&mut self, len: Idx) -> Option<LazyArray<'a, T, Idx>> {
        let len = len.to_usize() * T::SIZE;
        let len = u32::try_from(len).ok()?;
        let data = self.read_bytes(len)?;
        Some(LazyArray::new(data))
    }

    #[inline]
    pub fn read_array16<T: FromData>(&mut self) -> Option<LazyArray<'a, T, u16>> {
        let count: u16 = self.read()?;
        self.read_array(count)
    }

    pub fn read_array32<T: FromData>(&mut self) -> Option<LazyArray<'a, T, u32>> {
        let count: u32 = self.read()?;
        self.read_array(count)
    }
}


/// A "safe" stream.
///
/// Unlike `Stream`, `SafeStream` doesn't perform bounds checking on each read.
/// It leverages the type system, so we can sort of guarantee that
/// we do not read past the bounds.
///
/// For example, if we are iterating a `LazyArray` we already checked it's size
/// and we can't read past the bounds, so we can remove useless checks.
///
/// It's still not 100% guarantee, but it makes code easier to read and a bit faster.
/// And we still backed by the Rust's bounds checking.
#[derive(Clone, Copy, Default)]
pub struct SafeStream<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> SafeStream<'a> {
    #[inline]
    pub fn new(data: &'a [u8]) -> Self {
        SafeStream {
            data,
            offset: 0,
        }
    }

    #[inline]
    pub fn read<T: FromData>(&mut self) -> T {
        let start = self.offset;
        self.offset += T::SIZE;
        let end = self.offset;

        let data = &self.data[start..end];
        T::parse(data)
    }
}


pub trait Offset {
    fn to_usize(&self) -> usize;
    fn is_null(&self) -> bool { self.to_usize() == 0 }
}


#[derive(Clone, Copy, Debug)]
pub struct Offset16(pub u16);

impl Offset for Offset16 {
    fn to_usize(&self) -> usize {
        usize::from(self.0)
    }
}

impl FromData for Offset16 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        Offset16(SafeStream::new(data).read())
    }
}

impl FromData for Option<Offset16> {
    const SIZE: usize = Offset16::SIZE;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        let offset = Offset16::parse(data);
        if offset.0 != 0 { Some(offset) } else { None }
    }
}


#[derive(Clone, Copy, Debug)]
pub struct Offset32(pub u32);

impl Offset for Offset32 {
    #[inline]
    fn to_usize(&self) -> usize {
        usize::num_from(self.0)
    }
}

impl FromData for Offset32 {
    #[inline]
    fn parse(data: &[u8]) -> Self {
        Offset32(SafeStream::new(data).read())
    }
}


impl FromData for Option<Offset32> {
    const SIZE: usize = Offset32::SIZE;

    #[inline]
    fn parse(data: &[u8]) -> Self {
        let offset = Offset32::parse(data);
        if offset.0 != 0 { Some(offset) } else { None }
    }
}
