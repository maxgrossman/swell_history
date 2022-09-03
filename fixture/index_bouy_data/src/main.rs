use chrono::TimeZone;
use chrono::offset::LocalResult;
use chrono_tz::{Tz,UTC};
use flate2::read::GzDecoder;
use std::env;
use std::fs::File;
use std::io::{Read,Write};
use std::collections::HashMap;
use std::path::Path;
use rusqlite::{Connection, NO_PARAMS};


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

fn main() -> () {
    let mut args_vec: Vec<String> = env::args().collect::<Vec<String>>();
    let bouys_db: String = args_vec.pop().unwrap();
    let mut connection = Connection::open(bouys_db).unwrap();
    // get list of bouys, their timezone ids, and all the known history files
    let mut stmt = connection.prepare(
        "
        SELECT bouys.id, bouys.tzid, group_concat(bouy_history.filename)
        FROM bouys join bouy_history on bouys.id=bouy_history.bouy_id
        WHERE bouys.tzid != ''
        GROUP BY bouys.id;
        "
    ).unwrap();

    let mut rows = stmt.query(NO_PARAMS).unwrap();

    let history_base_url = "https://www.ndbc.noaa.gov/data/historical/stdmet";

    let client = reqwest::blocking::Client::builder()
        .gzip(true)
        .build()
        .unwrap();

    while let Some(res_row) = rows.next().unwrap() {
        let bouy_id: String = res_row.get(0).unwrap();
        let tzid: String = res_row.get(1).unwrap();
        let history_files: String = res_row.get(2).unwrap();

        let bouy_db = &format!("{}.sqlite", bouy_id);
        let db: &Path = Path::new(bouy_db);
        let db_exists = db.exists();

        let mut bouy_db_connection = Connection::open(bouy_db).unwrap();
        if !db_exists {
            bouy_db_connection.execute_batch("
                BEGIN;
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
                COMMIT;
          ").unwrap();
        }

        println!("Indexing {}", bouy_id);

        for history_file in history_files.as_str().split(",").collect::<Vec<&str>>() {
            let mut bouy_statement = String::from("BEGIN TRANSACTION;");
            let mut indexes: HashMap<String, usize> = HashMap::new();
            let bytes = client.get(String::from(format!("{base}/{file}", base=history_base_url, file=history_file)))
                .send().unwrap()
                .bytes().unwrap();

            let b: &[u8] = &bytes.to_vec();
            let mut gz = GzDecoder::new(b);
            let mut table = String::new();
            gz.read_to_string(&mut table).unwrap();

            let timezone: Tz = tzid.parse().unwrap();
            let mut have_min = false;
            let year_keys: [&str;2] = ["YY","YYYY"];
            let mut year_key_idx: usize = 0;

            for (i,line) in table.lines().enumerate() {
                if i == 0 {
                    for (i, header) in line.split_whitespace().enumerate() {
                        indexes.insert(String::from(header.replace("#","")),i);
                    }
                    if indexes.contains_key("YYYY") { year_key_idx += 1; }
                    have_min = indexes.contains_key("mm");
                } else if i > 1 {
                    let data: Vec<&str> = line.split_whitespace().collect();
                    let swell_direction = data[*indexes.get("MWD").unwrap()].parse::<f32>().unwrap();
                    if swell_direction < 0.0 || 360.00 < swell_direction {
                        continue
                    }

                    let maybe_timezone_ymd = timezone.ymd_opt(
                        data[*indexes.get(year_keys[year_key_idx]).unwrap()].parse::<i32>().unwrap(),
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
                        if have_min { data[*indexes.get("mm").unwrap()].parse::<u32>().unwrap() } else { 0 },
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

                    bouy_statement.push_str(format!(
                        "\nINSERT OR IGNORE INTO timestamps values ('{timestamp}',{dhp},{dp},{dh},{hp},{d},{h},{p});",
                        dhp=dhp_zid,dp=dp_zid,dh=dh_zid,
                        hp=hp_zid,d=swell_direction,
                        h=wave_height,p=swell_period,timestamp=utc_string
                    ).as_str())
                };
            }
            bouy_statement.push_str("\nCOMMIT;");
            bouy_db_connection.execute_batch(bouy_statement.as_str()).unwrap();
        }
    }
    File::create("~/created_bouys").unwrap().write_all(b".").unwrap();
}

