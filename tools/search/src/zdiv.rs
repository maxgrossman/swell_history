use std::iter::Iterator;


// 1 set for first 21 bits
const MAX_NUMB_BITS: u64 = 21;
const MAX_BITS_MASK: u64 = 0x1fffff;

struct ZDivRange {
    min: u64,
    max: u64,
    dims: u64,
    max_misses: usize,
    have_next: bool,
    morton_encoding: u64
}

impl ZDivRange {
    pub fn new (min: u64, max: u64, dims: u64) -> Self {
        Self { min: min, max:max, dims:dims, max_misses: 5, have_next: false, morton_encoding: min }
    }
    fn split(&self, to_split: u64) -> u64 {
        let mut split_num = to_split & MAX_BITS_MASK;

        split_num = (split_num ^ (split_num << 32)) & 0x00000000ffffffff;
        split_num = (split_num ^ (split_num << 16)) & 0x0000ffff0000ffff;
        split_num = (split_num ^ (split_num <<  8)) & 0x00ff00ff00ff00ff;
        split_num = (split_num ^ (split_num <<  4)) & 0x0f0f0f0f0f0f0f0f;
        split_num = (split_num ^ (split_num <<  2)) & 0x3333333333333333;
        split_num = (split_num ^ (split_num <<  1)) & 0x5555555555555555;

        return split_num;
    }
    fn bit_on_at_idx(&self, x: u64, idx: u64) -> u64 {
        // x & 1 left shifted to the idx bit, right shifted back over to 1st bit
        // when idx bit is turned on, this ret 1. when it is not, this ret 0
        (x & (1 << idx)) >> idx
    }
    fn load_zdivide_bits_into_target(&self, target: u64, over_under_bits: u64, bits: u64, dimension: u64) -> u64 {
        // mask here is all bits on except for dim's bit
        let bit_mask = !(self.split(MAX_BITS_MASK >> (MAX_NUMB_BITS - bits)) << dimension);
        // flips the bit, off, at dim
        let flipped_off_at_dim = target & bit_mask;
        // mask with bit seq 1000 or 0111 starting at index 'bits'
        let over_under_bits_at_dim = self.split(over_under_bits) << dimension;

        return flipped_off_at_dim | over_under_bits_at_dim;
    }
    fn over(&self, bits: u64) -> u64 { return 1u64 << (bits - 1) }
    fn under(&self, bits: u64) -> u64 { return (1u64 << (bits - 1)) - 1 }
    fn zdivide(&self) -> (u64, u64) {
        let mut i = 64;
        let mut zmin: u64 = self.min;
        let mut zmax: u64 = self.max;
        let mut litmax: u64 = 0;
        let mut bigmin: u64 = 0;

        while i > 0 {
            i -= 1;

            let bits: u64 = i / self.dims + 1;
            let dim = i % self.dims;
            let bits_on: (u64,u64,u64) = (
              self.bit_on_at_idx(self.morton_encoding, i),
              self.bit_on_at_idx(zmin, i),
              self.bit_on_at_idx(zmax, i)
            );

            match bits_on {
                (0, 0, 0) => {},
                (0, 0, 1) => {
                  zmax   = self.load_zdivide_bits_into_target(zmax, self.under(bits), bits, dim);
                  bigmin = self.load_zdivide_bits_into_target(zmin, self.over(bits), bits, dim);
                },
                (0, 1, 0) => {},
                (0, 1, 1) => {
                  bigmin = zmin;
                  return (litmax, bigmin)
                },
                (1, 0, 0) => {
                  litmax = zmax;
                  return (litmax, bigmin)
                },
                (1, 0, 1) => {
                  litmax = self.load_zdivide_bits_into_target(zmax, self.under(bits), bits, dim);
                  zmin = self.load_zdivide_bits_into_target(zmin, self.over(bits), bits, dim);
                },
                (1, 1, 0) => {},
                (1, 1, 1) => {},
                _ => {}
            }

        }

        return (litmax, bigmin)
    }
    fn set_morton_encoding_to_bigmin(&mut self) -> () {
        let (litmax, bigmin) = self.zdivide();
        self.morton_encoding = bigmin;
    }
}

impl IntoIterator for ZDivRange {
    type Item = u64;
    type IntoIter = ZDivRangeIntoIterator;
    fn into_iter(self) -> Self::IntoIter {
        return ZDivRangeIntoIterator { zdiv_range: self };

    }

}

struct ZDivRangeIntoIterator {
    zdiv_range: ZDivRange,
}

impl Iterator for ZDivRangeIntoIterator {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        let mut misses = 0;
        let max: u64 = self.zdiv_range.max;
        let min: u64 = self.zdiv_range.min;
        while misses < self.zdiv_range.max_misses && self.zdiv_range.have_next {
            if min <= self.zdiv_range.morton_encoding &&
               self.zdiv_range.morton_encoding <= max {
               self.zdiv_range.have_next = true;
               self.zdiv_range.morton_encoding += 1;
               return Some(self.zdiv_range.morton_encoding);
            } else {
                misses += 1;
            }
        }
        if self.zdiv_range.morton_encoding < max {
            self.zdiv_range.set_morton_encoding_to_bigmin();
            self.zdiv_range.have_next = true;
            return Some(self.zdiv_range.morton_encoding);
        } else {
            self.zdiv_range.have_next = false;
            None
        }
    }
}

/**
 * Given minimum and maximum mortons codes inclusive of codes not in
 * a bounding box, replies set of ranges exclusive to only those in bounds.
 */
fn get_bbox_ranges(min: u64, max: u64, dimensions: u64) -> Vec<Vec<u64>> {
    let mut zdiv_range_iter: ZDivRangeIntoIterator =
        crate::zdiv::ZDivRange::new(min, max, dimensions).into_iter();

    let mut zdiv_ranges: Vec<Vec<u64>> = Vec::new();
    let mut range_index: usize = 0;
    let mut last: u64 = 0;

    match zdiv_range_iter.next() {
        Some(morton) => {
            last = morton;
            zdiv_ranges.push(vec![morton]);
        },
        None => {}
    }

    for morton in zdiv_range_iter.next() {
        if (last + 1) != morton {
            if zdiv_ranges[range_index][0] != last {
                zdiv_ranges[range_index].push(last);
            }
            range_index += 1;
        }
        last = morton;
    }

    if zdiv_ranges[range_index][0] != last {
        zdiv_ranges[range_index].push(last);
    }

    return zdiv_ranges;
}



#[cfg(test)]
mod tests {
    #[test]
    fn wikipedia_example() {
        let mut wiki_zrange =  crate::zdiv::ZDivRange::new(12u64, 45u64, 2);
        wiki_zrange.morton_encoding = 19u64;
        let (litmax, bigmin) = wiki_zrange.zdivide();
        assert_eq!(litmax, 15u64);
        assert_eq!(bigmin, 36u64);
    }
    #[test]
    fn wikipedia_example_bbox_ranges() {
        let bbox_ranges = crate::zdiv::get_bbox_ranges(12u64, 45u64, 2u64);
        println!("{:#?}",bbox_ranges[0]);
        assert_eq!(bbox_ranges[0][0], 12u64);  assert_eq!(bbox_ranges[0][1], 13u64);
        // assert_eq!(bbox_ranges[1][0], 14u64);  assert_eq!(bbox_ranges[1][1], 15u64);
        // assert_eq!(bbox_ranges[2][0], 36u64);  assert_eq!(bbox_ranges[2][1], 37u64);
        // assert_eq!(bbox_ranges[3][0], 44u64);  assert_eq!(bbox_ranges[3][1], 45u64);

    }
}
