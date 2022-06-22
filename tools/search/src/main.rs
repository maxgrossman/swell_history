use std::fs;
use std::num::ParseFloatError;
use std::env;
use std::collections::{HashMap,HashSet};
use std::path::Path;
use serde_json::Value;
use morton_encoding::morton_decode;

fn main() {
  let PATH: String = String::from("/Users/maxgrossman/Documents/github/maxgrossman/swell_history/bouys");
  let args: Vec<String> = env::args().collect();
  let mut swell_signatures: Vec<String> = vec![
    String::from("--h"),
    String::from("--p"),
    String::from("--d")
  ];

  // start, can i de interleave and check against h/p/d?
  let mut hpd: Vec<u32> = vec![0,0,0,0,0,0];
  let mut have_sigs = 0;

  for i in 1..args.len() - 1 {
    let signature_flag: Option<usize> = swell_signatures.iter().position(|a| a.eq(&args[i]));
    match signature_flag {
      Some(shift_pos) => {
        let query: Vec<u32> = args[i+1].split(",").map(|n| n.parse::<u32>().unwrap()).collect::<Vec<u32>>();
        hpd[2*shift_pos]     = query[0];
        hpd[2*shift_pos + 1] = match query.len() {
            1 => (query[0] + 1),
            _ => query[1]
        };

        if shift_pos < 2 {
            hpd[2*shift_pos] *= 100;
            hpd[2*shift_pos + 1] *= 100;
        }
        // where h is encoded by 1, p 2, and d 4, encode their encodings in have_sigs when matched
        have_sigs += 1 << shift_pos;
      },
      None => {}
    }
  }

  // let mut decoded_map: HashMap<u64,[u32;3]> = HashMap::new();

  match args.last() {
    Some(bouy_id) => {
      let _bouy_str_path = format!("{}/{}/{}", PATH, bouy_id, "index");
      let bouy_indexes_path = Path::new(&_bouy_str_path);
      if bouy_indexes_path.exists() {
        for entry in bouy_indexes_path.read_dir().expect("") {
          match entry {
            Ok(z_index) => {
              match z_index.file_name().into_string() {
               Ok(z_index_str) => {
                  let z_index_uint: u64 = z_index_str.parse::<u64>().unwrap();

                  // if !decoded_map.contains_key(&z_index_uint) {
                    // decoded_map.insert(z_index_uint, morton_decode(u128::from(z_index_uint)));
                  // }
                  let decoded: [u32;3] =  morton_decode(u128::from(z_index_uint));
                  // let decoded: &[u32; 3] = decoded_map.get(&z_index_uint).unwrap();

                  let mut matches = have_sigs > 0;
                  if have_sigs & 1 == 1 { matches = matches && hpd[0] <= decoded[2] && decoded[2] < hpd[1]; }
                  if have_sigs & 2 == 2 { matches = matches && hpd[2] <= decoded[1] && decoded[1] < hpd[3]; }
                  if have_sigs & 4 == 4 { matches = matches && hpd[4] <= decoded[0] && decoded[0] < hpd[5]; }

                  if matches == true {
                    let timestamps = fs::read_to_string(z_index.path());
                    match timestamps {
                        Ok(timestamps_str) => {
                            for timestamp in timestamps_str.lines() {
                                let reading_json = fs::read_to_string(format!("{}/{}/{}/{}", PATH, bouy_id, "readings", timestamp));
                                match reading_json {
                                    Ok(reading_string) => {
                                        let reading: HashMap<String,Value> = serde_json::from_str(&reading_string).unwrap();
                                        println!(
                                            "{} => height={} period={} direction={}",
                                            timestamp,
                                            reading.get("wvht").unwrap(),
                                            reading.get("apd").unwrap(),
                                            reading.get("mwd").unwrap(),
                                        );
                                    },
                                    Err(_) => {}
                                }
                            }
                        },
                        Err(_) => {}
                    }
                  }
                }
                Err(_) => {}
              }
            },
            Err(_) => {}
          }
        }
      }
    }
    None => {}
  }
}

// flags --direction, --height, --period
// arguments cld bouys
// use serde_json::Map;

