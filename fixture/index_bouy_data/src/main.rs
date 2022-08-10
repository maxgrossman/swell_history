use chrono::TimeZone;
use chrono::offset::LocalResult;
use chrono_tz::{Tz,UTC};
use flate2::read::GzDecoder;
use std::env;
use std::io::Read;
use std::collections::HashMap;
use std::path::Path;
use sqlite;

const MAX_BITS_MASK_3D: u64 = 0x1fffff;
const MAX_BITS_MASK_2D: u32 = 0x0000ffff;

fn load_into_bits_3d(mut x: u64) -> u64 {
    x &= MAX_BITS_MASK_3D;
    x = (x | x << 32) & 0x1f00000000ffff; 
    x = (x | x << 16) & 0x1f0000ff0000ff; 
    x = (x | x << 8)  & 0x100f00f00f00f00f; 
    x = (x | x << 4)  & 0x10c30c30c30c30c3; 
    x = (x | x << 2)  & 0x1249249249249249;
    return x as u64;
}

fn load_into_bits_2d(mut x: u32) -> u32 {
    x &= MAX_BITS_MASK_2D;
    x = (x | (x << 8))  & 0x00ff00ff; 
    x = (x | (x << 4))  & 0x0f0f0f0f;
    x = (x | (x << 2))  & 0x33333333; 
    x = (x | (x << 1))  & 0x55555555;
    return x
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: HashMap<String,String> = env::args().into_iter()
        .filter(|arg| arg.contains("="))
        .map(|arg| {
            let split: Vec<&str> = arg.split("=").collect();
            (String::from(split[0]),String::from(split[1]))
        })
        .collect();

    let timezone_str = args.get("timezone");
    let url = args.get("url");
    let bouy_id = args.get("bouy"); 

    if timezone_str.is_none() || url.is_none() || bouy_id.is_none() {
        panic!("missing required arguments timezone, url, and bouy");
    }

    // "America/California";
    // "https://www.ndbc.noaa.gov/data/historical/stdmet/46224h2021.txt.gz";
    let client_result = reqwest::blocking::Client::builder()
        .gzip(true)
        .build();

    match client_result {
        Ok(client) => {
            let bytes = client.get(url.unwrap()).send()?.bytes()?;
            let b: &[u8] = &bytes.to_vec();
            let mut gz = GzDecoder::new(b);
            let mut table = String::new();
            gz.read_to_string(&mut table)?;

            let mut indexes: HashMap<String, usize> = HashMap::new();
            let db_path_str = &format!("{}.sqlite", bouy_id.unwrap());
            let db: &Path =  Path::new(db_path_str);
            let db_exists: bool = db.exists();
            let connection = sqlite::open(db.to_str().unwrap()).unwrap();


            connection.execute("BEGIN TRANSACTION;").unwrap();
            if !db_exists {
                connection.execute(
                    "
                    CREATE TABLE timestamps (reading_time timestamp primary key,
                                             dhp bigint,
                                             dh  bigint,
                                             dp  bigint,
                                             hp  bigint,
                                             d   bigint,
                                             h   bigint,
                                             p   bigint);

                    CREATE INDEX indx_dhp on timestamps(dhp);
                    CREATE INDEX indx_dh on timestamps(dh);
                    CREATE INDEX indx_dp on timestamps(dp);
                    CREATE INDEX indx_hp on timestamps(hp);
                    CREATE INDEX indx_d on timestamps(d);
                    CREATE INDEX indx_h on timestamps(h);
                    CREATE INDEX indx_p on timestamps(p);
                    "
                )
                .unwrap();
            }
            //TODO: FIGURE OUT HOW CHRONO_TZ MAKES ITS TIMEZONE NAMES AND THEN FIGURE OUT IF
            //THERE'S SOFTWARE OUT THERE THAT I USE TO RETRIEVE A NAME GIVEN A LON/LAT
            let timezone: Tz = timezone_str.unwrap().parse().unwrap();
            for (i,line) in table.lines().enumerate() {
                if i == 0 {
                    for (i, header) in line.split_whitespace().enumerate() {
                        indexes.insert(String::from(header.replace("#","")),i);
                    }
                } else if i > 1 {
                    let data: Vec<&str> = line.split_whitespace().collect();
                    let swell_direction = data[*indexes.get("MWD").unwrap()].parse::<f32>().unwrap();
                    if swell_direction < 0.0 || 360.00 < swell_direction {
                        continue
                    }
                    //TODO: this will come from function param

                    let maybe_timezone_ymd = timezone.ymd_opt(
                        data[*indexes.get("YY").unwrap()].parse::<i32>().unwrap(),
                        data[*indexes.get("MM").unwrap()].parse::<u32>().unwrap(),
                        data[*indexes.get("DD").unwrap()].parse::<u32>().unwrap()
                    );

                    let timezone_ymd = match maybe_timezone_ymd {
                        LocalResult::Single(timezone_ymd) => timezone_ymd,
                        LocalResult::Ambiguous(_,_) => continue,
                        LocalResult::None => continue
                    };

                    let maybe_timezone_ymd_hms = timezone_ymd.and_hms_opt(
                        data[*indexes.get("hh").unwrap()].parse::<u32>().unwrap(),
                        data[*indexes.get("mm").unwrap()].parse::<u32>().unwrap(),
                        0
                    );

                    let utc_string = match maybe_timezone_ymd_hms {
                        Some(timezone_yms_hms) => timezone_yms_hms.with_timezone(&UTC).to_string(),
                        None => continue
                    };

                    let swell_direction_int: u32 = swell_direction as u32;
                    let swell_period: u32 = (data[*indexes.get("APD").unwrap()].parse::<f32>().unwrap() * 100.0) as u32;
                    let wave_height: u32 = (data[*indexes.get("WVHT").unwrap()].parse::<f32>().unwrap() * 100.0) as u32;

                    let dhp_zid: u64 = 
                        load_into_bits_3d(swell_direction_int as u64) |
                        load_into_bits_3d(wave_height as u64) >> 1    |
                        load_into_bits_3d(swell_period as u64) >> 2;

                    let dh_zid: u32 =
                        load_into_bits_2d(swell_direction_int) |
                        load_into_bits_2d(wave_height) >> 1;

                    let dp_zid: u32 = 
                        load_into_bits_2d(swell_direction_int) |
                        load_into_bits_2d(swell_period) >> 1;

                    let hp_zid: u32 = 
                        load_into_bits_2d(wave_height) |
                        load_into_bits_2d(swell_period) >> 1;

                    connection.execute(format!(
                        "INSERT INTO TIMESTAMPS values ('{timestamp}',{dhp},{dp},{dh},{hp},{d},{h},{p});",
                        dhp=dhp_zid,dp=dp_zid,dh=dh_zid,
                        hp=hp_zid,d=swell_direction,
                        h=wave_height,p=swell_period,timestamp=utc_string
                    )).unwrap()
                };
            }
            connection.execute("COMMIT").unwrap();
        },
        Err(err) => {
            println!("Error: {}", err);
        }
    }
    Ok(())
}

