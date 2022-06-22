use node_bindgen::derive::node_bindgen;
use morton_encoding::morton_encode;


#[node_bindgen]
fn interleave(wave_height: u32, swell_period: u32, swell_direction: u32) -> String {
    return u64::try_from(
        morton_encode([
            swell_direction & 0x1fffff,
            swell_period & 0x1fffff,
            wave_height & 0x1fffff      // make sure we only use first 21 bits
        ]) & 0xffffffffffffffff         // mask to only first 64 bits
    ).unwrap().to_string();
}
