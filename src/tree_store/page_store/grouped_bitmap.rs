// A bitmap which groups consecutive groups of 64bits together

use std::mem::size_of;

pub(crate) struct U64GroupedBitMap<'a> {
    data: &'a [u8],
}

impl<'a> U64GroupedBitMap<'a> {
    pub(crate) fn new(data: &'a [u8]) -> Self {
        assert_eq!(data.len() % 8, 0);
        Self { data }
    }

    pub(crate) fn count_unset(&self) -> usize {
        self.data.iter().map(|x| x.count_zeros() as usize).sum()
    }
}

pub(crate) struct U64GroupedBitMapMut<'a> {
    data: &'a mut [u8],
}

impl<'a> U64GroupedBitMapMut<'a> {
    pub(crate) fn required_bytes(elements: usize) -> usize {
        let words = (elements + 63) / 64;
        words * size_of::<u64>()
    }

    pub(crate) fn new(data: &'a mut [u8]) -> Self {
        assert_eq!(data.len() % 8, 0);
        Self { data }
    }

    pub(crate) fn len(&self) -> usize {
        self.data.len() * 8
    }

    // Returns true iff the bit's group is all set
    pub(crate) fn set(&mut self, bit: usize) -> bool {
        let (index, bit_index) = self.data_index_of(bit);
        let mut group = u64::from_le_bytes(self.data[index..(index + 8)].try_into().unwrap());
        group |= Self::select_mask(bit_index);
        self.data[index..(index + 8)].copy_from_slice(&group.to_le_bytes());

        group == u64::MAX
    }

    pub(crate) fn get(&self, bit: usize) -> bool {
        let (index, bit_index) = self.data_index_of(bit);
        let group = u64::from_le_bytes(self.data[index..(index + 8)].try_into().unwrap());
        group & Self::select_mask(bit_index) != 0
    }

    pub(crate) fn clear(&mut self, bit: usize) {
        let (index, bit_index) = self.data_index_of(bit);
        let mut group = u64::from_le_bytes(self.data[index..(index + 8)].try_into().unwrap());
        group &= !Self::select_mask(bit_index);
        self.data[index..(index + 8)].copy_from_slice(&group.to_le_bytes());
    }

    pub(crate) fn first_unset(&self, start_bit: usize, end_bit: usize) -> Option<usize> {
        assert_eq!(start_bit % 64, 0);
        assert_eq!(end_bit, start_bit + 64);

        let (index, _) = self.data_index_of(start_bit);
        let group = u64::from_le_bytes(self.data[index..(index + 8)].try_into().unwrap());
        match group.trailing_ones() {
            64 => None,
            x => Some(start_bit + x as usize),
        }
    }

    fn data_index_of(&self, bit: usize) -> (usize, usize) {
        ((bit / 64) as usize * size_of::<u64>(), (bit % 64) as usize)
    }

    fn select_mask(bit: usize) -> u64 {
        1u64 << (bit as u64)
    }
}
