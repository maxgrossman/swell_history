use std::iter::Iterator;

// 1 set for first 21 bits
const MAX_NUMB_BITS: u64 = 21;
const MAX_BITS_MASK: u64 = 0x1fffff;

fn load_into_bits_2d(value: u32) -> u64 {
    let mut masked: u64 = (value as u64) & MAX_BITS_MASK;
    masked = (masked ^ (masked << 32)) & 0x00000000ffffffff;
    masked = (masked ^ (masked << 16)) & 0x0000ffff0000ffff;
    masked = (masked ^ (masked <<  8)) & 0x00ff00ff00ff00ff;
    masked = (masked ^ (masked <<  4)) & 0x0f0f0f0f0f0f0f0f;
    masked = (masked ^ (masked <<  2)) & 0x3333333333333333;
    masked = (masked ^ (masked <<  1)) & 0x5555555555555555;
    return masked;
}

fn gather_dim_bits(morton_value: u64) -> u32 {
        let mut masked: u64 = morton_value & 0x5555555555555555u64;
        masked = (masked ^ (masked >>  1)) & 0x3333333333333333u64;
        masked = (masked ^ (masked >>  2)) & 0x0f0f0f0f0f0f0f0fu64;
        masked = (masked ^ (masked >>  4)) & 0x00ff00ff00ff00ffu64;
        masked = (masked ^ (masked >>  8)) & 0x0000ffff0000ffffu64;
        masked = (masked ^ (masked >> 16)) & 0x00000000ffffffffu64;
        return masked as u32;
}

fn break_into_array_2d(morton_value: u64) -> [u32;2] {
    return [
        gather_dim_bits(morton_value),
        gather_dim_bits(morton_value) >> 1
    ]
}

struct ZDiv2dRange {
    min_decoded: [u32;2],
    min: u64,
    max_decoded: [u32;2],
    max: u64,
    dims: u64,
    morton_encoding: u64,
    morton_decoded: [u32;2]
}

impl ZDiv2dRange {
    pub fn new (min_decoded: [u32;2], max_decoded: [u32;2]) -> Self {
        let min = load_into_bits_2d(min_decoded[0]) | load_into_bits_2d(min_decoded[1]) << 1;
        let max = load_into_bits_2d(max_decoded[0]) | load_into_bits_2d(max_decoded[1]) << 1;
        Self {
            min_decoded: min_decoded,
            max_decoded: max_decoded,
            min: min,
            max: max,
            morton_encoding: min,
            morton_decoded: min_decoded,
            dims: 2
        }
    }
    fn merge_every_other_bit(&self, morton_value: u64) -> u32 {
        let mut masked: u64 = morton_value & 0x5555555555555555u64;
        masked = (masked ^ (masked >>  1)) & 0x3333333333333333u64;
        masked = (masked ^ (masked >>  2)) & 0x0f0f0f0f0f0f0f0fu64;
        masked = (masked ^ (masked >>  4)) & 0x00ff00ff00ff00ffu64;
        masked = (masked ^ (masked >>  8)) & 0x0000ffff0000ffffu64;
        masked = (masked ^ (masked >> 16)) & 0x00000000ffffffffu64;
        return masked as u32;

    }
    fn get_morton_array_2d(&self, morton_value: u64) -> [u32;2] {
        return [
            self.merge_every_other_bit(morton_value),
            self.merge_every_other_bit(morton_value) >> 1,
        ]
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
        self.morton_decoded = self.get_morton_array_2d(bigmin);
    }


    fn in_bounds(&mut self) -> bool {
        return self.min_decoded[0] <= self.morton_decoded[0] && // morton x greater than eq min x
               self.max_decoded[0] >= self.morton_decoded[0] && // morton x less than eq max x
               self.min_decoded[1] <= self.morton_decoded[1] && // morton y greater than eq min y
               self.max_decoded[1] >= self.morton_decoded[1];   // morton y less than eq max y
    }
}

impl IntoIterator for ZDiv2dRange {
    type Item = u64;
    type IntoIter = ZDiv2dRangeIntoIterator;
    fn into_iter(self) -> Self::IntoIter {
        let have_next = self.min < self.max;
        return ZDiv2dRangeIntoIterator {
            zdiv_range: self,
            max_misses: 5,
            have_next: have_next
        };
    }

}

struct ZDiv2dRangeIntoIterator {
    zdiv_range: ZDiv2dRange,
    max_misses: usize,
    have_next: bool
}

impl Iterator for ZDiv2dRangeIntoIterator {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        let mut misses = 0;
        while misses < self.max_misses && self.zdiv_range.morton_encoding <= self.zdiv_range.max {
            self.zdiv_range.morton_encoding += 1;
            self.zdiv_range.morton_decoded = 
                self.zdiv_range.get_morton_array_2d(self.zdiv_range.morton_encoding);
            if self.zdiv_range.in_bounds() {
                self.have_next = true;
                return Some(self.zdiv_range.morton_encoding);
            } else {
                misses += 1;
            }
        }

        if self.zdiv_range.morton_encoding < self.zdiv_range.max {
            self.have_next = true;
            self.zdiv_range.set_morton_encoding_to_bigmin();
            return Some(self.zdiv_range.morton_encoding);
        } else {
            self.have_next = false;
            None
        }
    }
}

/**
 * Given minimum and maximum mortons codes inclusive of codes not in
 * a bounding box, replies set of ranges exclusive to only those in bounds.
 */
fn get_2d_bbox_ranges(min_pnt: [u32;2], max_pnt: [u32;2]) -> Vec<Vec<u64>> {
    let mut zdiv_range_iter: ZDiv2dRangeIntoIterator =
        crate::zdiv::ZDiv2dRange::new(min_pnt, max_pnt).into_iter();

    let mut zdiv_ranges: Vec<Vec<u64>> = Vec::new();
    let mut range_index: usize = 0;
    let mut last: u64 = zdiv_range_iter.zdiv_range.morton_encoding;

    zdiv_ranges.push(vec![zdiv_range_iter.zdiv_range.morton_encoding]);

    while let Some(morton) = zdiv_range_iter.next() {
        println!("{}", morton);
        if (last + 1) != morton {
            match zdiv_ranges.get(range_index) {
                Some(range) => {
                    match range.get(0) {
                        Some(first) => {
                            if *first != last {
                                zdiv_ranges[range_index].push(last);
                            }
                        },
                        None => {}
                    }
                },
                None => panic!("Looking for out of index range")
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
        let mut wiki_zrange =  crate::zdiv::ZDiv2dRange::new([2,2], [3,6]);
        wiki_zrange.morton_encoding = 19u64;
        let (litmax, bigmin) = wiki_zrange.zdivide();
        assert_eq!(litmax, 15u64);
        assert_eq!(bigmin, 36u64);
    }
    #[test]
    fn wikipedia_example_bbox_ranges() {
        let bbox_ranges = crate::zdiv::get_2d_bbox_ranges([5,3], [10,5]);
        assert_eq!(bbox_ranges[0][0], 12u64);  assert_eq!(bbox_ranges[0][1], 13u64);
        // assert_eq!(bbox_ranges[1][0], 14u64);  assert_eq!(bbox_ranges[1][1], 15u64);
        // assert_eq!(bbox_ranges[2][0], 36u64);  assert_eq!(bbox_ranges[2][1], 37u64);
        // assert_eq!(bbox_ranges[3][0], 44u64);  assert_eq!(bbox_ranges[3][1], 45u64);

    }
}
