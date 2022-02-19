/// Represents Minecraft's packed bit arrays.
/// These caused me so many headaches :/
pub struct PackedBitArray {
    data: Vec<u64>,
    bits_per_entry: usize,
    values_per_long: usize,
}

impl PackedBitArray {
    /// Constructs a new PackedBitArray.
    /// Assumes that the data contains 4096 entries, and will panic otherwise
    pub fn new(data: Vec<u64>, palette_size: usize) -> Self {
        let bits_per_entry = Self::compute_bits_per_entry(palette_size);
        let values_per_long = 64 / bits_per_entry;
        assert_eq!((4096.0 / values_per_long as f64).ceil() as usize, data.len(), "data size does not match expected");

        Self {
            data,
            bits_per_entry,
            values_per_long,
        }
    }

    pub fn empty(palette_size: usize) -> Self {
        let bits_per_entry = Self::compute_bits_per_entry(palette_size);
        let values_per_long = 64 / bits_per_entry;
        let data_length = (4096.0 / values_per_long as f64).ceil() as usize;
        Self {
            data: vec![0; data_length],
            bits_per_entry,
            values_per_long,
        }
    }

    // bits_per_entry = ceil(log2(palette_size))
    fn compute_bits_per_entry(palette_size: usize) -> usize {
        let bpe = (palette_size as f64).log2().ceil() as usize;
        if bpe < 4 {
            4
        } else {
            bpe
        }
    }

    pub fn get_value(&self, i: usize) -> u64 {
        let long_idx = i / self.values_per_long;
        let long_offset = (i % self.values_per_long) * self.bits_per_entry;
        let mask = self.create_mask(long_offset);
        (self.data[long_idx] & mask) >> long_offset
    }

    pub fn put_value(&mut self, index: usize, value: u64) {
        // ensure that we cannot overwrite any unexpected data.
        assert_eq!(value & mask(self.bits_per_entry), value, "value does not fit");
        let long_idx = index / self.values_per_long;
        let long_offset = (index % self.values_per_long) * self.bits_per_entry;
        self.data[long_idx] |= value << long_offset;
    }

    pub fn data(&self) -> &[u64] {
        &self.data
    }

    pub fn bits_per_entry(&self) -> usize {
        self.bits_per_entry
    }

    fn create_mask(&self, offset_in_long: usize) -> u64 {
        let far_length = offset_in_long + self.bits_per_entry - 1;
        mask(far_length) ^ mask(offset_in_long)
    }
}

#[inline]
fn mask(length: usize) -> u64 {
    if length == 64 {
        u64::MAX
    } else {
        (1 << length) - 1
    }
}
