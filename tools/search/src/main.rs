mod zdiv;
use std::env;
use std::path::Path;
// use serde_json::Value;
// use morton_encoding::morton_encode_generic_checked;

fn main() {
  let path: String = String::from("/Users/maxgrossman/Documents/github/maxgrossman/swell_history/bouys");
  let args: Vec<String> = env::args().collect();
  let swell_signatures: Vec<String> = vec![
    String::from("--d"),
    String::from("--h"),
    String::from("--p")
  ];

  // start, can i de interleave and check against h/p/d?
  let mut dhp: [Vec<u32>;2] = [Vec::new(), Vec::new()];
  let mut have_sigs = 0;

  for i in 1..args.len() - 1 {
    let signature_flag: Option<usize> = swell_signatures.iter().position(|a| a.eq(&args[i]));
    match signature_flag {
      Some(shift_pos) => {
        let query: Vec<u32> = args[i+1].split(",").map(|n| n.parse::<u32>().unwrap()).collect::<Vec<u32>>();
        dhp[0].push(query[0] * 100);
        dhp[1].push(query[1] * 100);
        // where h is encoded by 1, p 2, and d 4, encode their encodings in have_sigs when matched
        have_sigs += 1 << shift_pos;
      },
      None => {}
    }
  }

  match args.last() {
    Some(bouy_id) => {
        let bouy_db_str = format!("{}/{}.sqlite", path, bouy_id);
        let bouy_db_path = Path::new(&bouy_db_str);
        if bouy_db_path.exists() {
            // create by bbox, knowing what index i am going to look through
            let mut col_name: String = String::new();

            if have_sigs & 1 == 1 { col_name.push_str("d"); }
            if have_sigs & 2 == 2 { col_name.push_str("h"); }
            if have_sigs & 4 == 4 { col_name.push_str("p"); }


            let mut statement: String = String::new();

            if col_name.len() == 1 {
                statement.push_str(&format!(
                   "SELECT reading_time FROM timestamps
                    WHERE {min} <= {col_name} AND {col_name} <= {max};",
                    col_name=col_name,
                    min=dhp[0][0],
                    max=dhp[1][1]
                ))
            } else {
                // let bound_min: Option<u64> = morton_encode_generic_checked::<_,u64,_>(dhp[0].clone());
                // let bound_max: Option<u64> = morton_encode_generic_checked::<_,u64,_>(dhp[1].clone());
            }
        }
    },
    None => {}
  }
}
                //let range_nums: Vec<u64> = Vec::new();
                //let first = bounds_inputs[0];
                //let num_dimensions = dhp[0].len();
                //let missed_count = 0;
                //let max_missed = 3;
                //for x in bounds_slices[0]..bounds_slices[1] {
                //    let decoded: Vec<u16> = morton_decode_generic(x, num_dimensions);
                //    let inside = 0;
                //    for (pos, x) in decoded.iter().enumerate() {
                //        if (x < dhp[0][pos] || x > dhp[1][pos]) {
                //            missed_count++;
                //            break;
                //        } else {
                //            ++inside;
                //        }
                //    }
                //    if (inside == decoded.len() {
                //        range_nums.append(x);
                //    } else if missed_count == max_missed {
                //        missed_count = 0
                //        // find the most significant bit, the largest one, that is different.
                //        let diff_input = bounds_slice[0] ^ bounds_slices[1];
                //        let diff_source_pos = (log2(diff_input & -diff_input) + 1);

                //        // then right shift that bit until it is in the bounds of the first set of interleaven bits
                //        // this tells us which dimension by its index.
                //        let slice_dimension = diff_source >> (num_dimensions * (diff_source_pos / num_dimension))
                //        let litmax: [u64;3] = [0,0,0];
                //        let bigmin: [u64;3] = [0,0,0];

                //        litmax[slice_dimension] = bounds_inputs[1][slice_dimension];
                //        bigmin[slice_dimension] = bonnds_inputs[0][slice_dimension];



                //        //BIG_MIN, find me next smallest value back in my box
                //        //LIT_MAX, find me next largest  value back in my box
                //    }


                    // how am i seeing if it is inside the encoded box? do i de-interleave and
                    // check?

// flags --direction, --height, --period
// arguments cld bouys
// use serde_json::Map;

