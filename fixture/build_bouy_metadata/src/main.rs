use std::env::args;
use std::f64::consts::{PI,E};
use serde_json::{Value};
use rusqlite::{Connection, NO_PARAMS};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use regex::Regex;

// built referencing https://github.com/mapbox/tilebelt pointToTile
const D2R: f64 = PI / 180.0;
fn point_to_tile_x_y(lon: f64, lat: f64, z: u32) -> [u64;2] {
    let sin: f64 = (lat * D2R).sin();
    let z2: f64 = f64::from(2u32.pow(z));
    let mut x: f64 = z2 * (lon / 360.0 + 0.5);
    let y: f64 = z2 * (0.5 - 0.25 * ((1.0_f64 + sin) / (1.0_f64 - sin)).log(E) / PI);
    x %= z2;
    if x < 0_f64 { x += z2 };
    return [(x.floor() as u64), (y.floor() as u64)];
}

fn main() {
    let mut args_vec: Vec<String> = args().collect::<Vec<String>>();
    let bouys_db: String = args_vec.pop().unwrap();
    let bouys_history_file_path: String = args_vec.pop().unwrap();
    let timezones_db: String = args_vec.pop().unwrap();
    let mut connection = Connection::open(bouys_db).unwrap();
    let client = reqwest::blocking::Client::builder().build().unwrap();
    let resp = client.get("https://www.ndbc.noaa.gov/ndbcmapstations.json").send().unwrap();
    let bouys_json: Value = serde_json::from_str(resp.text().unwrap().as_str()).unwrap();
    let bouy_name_regex = Regex::new(r"(h19|h20)").unwrap();
    unsafe {
        connection.load_extension_enable();
        let r = connection.load_extension("mod_spatialite", None);
        connection.execute(format!("ATTACH \"{db}\" AS tzdb;",db=timezones_db).as_str(), NO_PARAMS).unwrap();
        let mut bouys_statement = String::from(
            "
            BEGIN TRANSACTION;
            SELECT InitSpatialMetaData();
            CREATE TABLE bouys(id text, tzid text);
            SELECT AddGeometryColumn('bouys', 'geometry',  4326, 'POINT', 'XY');
            CREATE TABLE bouy_tile_indexes(bouy_id text, zoom integer, x integer, y integer, FOREIGN KEY(bouy_id) REFERENCES bouys(id));
            CREATE TABLE bouy_history(bouy_id text, filename text, FOREIGN KEY(bouy_id) REFERENCES bouys(id));
            CREATE INDEX idx_bouy_tile ON bouy_tile_indexes (bouy_id);
            CREATE INDEX idx_history ON bouy_history (bouy_id);
            "
        );
        for station in bouys_json["station"].as_array().unwrap() {
            let lon = station["lon"].as_f64().unwrap();
            let lat = station["lat"].as_f64().unwrap();
            let bouy_id: String = String::from(station["id"].as_str().unwrap());
            bouys_statement.push_str(format!(
                "\nINSERT INTO bouys values ('{bouy_id}', '', GeomFromText('POINT({lon} {lat})', 4326));",
                bouy_id=bouy_id, lon=lon, lat=lat
            ).as_str());

            for zoom in 0..21 {
                let tile_x_y: [u64;2] = point_to_tile_x_y(lon, lat, zoom);
                bouys_statement.push_str(format!(
                    "\nINSERT INTO bouy_tile_indexes values ('{bouy_id}', {zoom}, {x}, {y});",
                    bouy_id=bouy_id, zoom=zoom, x=tile_x_y[0], y=tile_x_y[1]
                ).as_str());
            }
        }

        let file = File::open(bouys_history_file_path).unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let bouy_file: String = line.unwrap();
            let bouy_id = String::from(bouy_name_regex.split(bouy_file.as_str()).next().unwrap());
            bouys_statement.push_str(format!(
                "\nINSERT INTO bouy_history values ('{bouy_id}', '{filename}');",
                bouy_id=bouy_id, filename=bouy_file
            ).as_str());
        }

        bouys_statement.push_str("SELECT CreateSpatialIndex('bouys','geometry');\nCOMMIT;");

        connection.execute_batch(bouys_statement.as_str()).unwrap();

        let mut stmt = connection.prepare(
            "
            SELECT tzdb.timezones.tzid as tzid, bouys.id as id
            FROM bouys, tzdb.timezones
            WHERE within(bouys.geometry, tzdb.timezones.geometry) = 1
            AND bouys.ROWID IN (
                SELECT ROWID
                FROM SpatialIndex
                WHERE f_table_name = 'bouys'
                AND search_frame = tzdb.timezones.geometry
            )
            "
        ).unwrap();

        let mut rows = stmt.query(NO_PARAMS).unwrap();
        let mut tzid_insert_stmt = String::from("BEGIN TRANSACTION;\n");

        while let Some(res_row) = rows.next().unwrap() {
            let tzid: String = res_row.get(0).unwrap();
            let bouy_id: String = res_row.get(1).unwrap();
            tzid_insert_stmt.push_str(
                format!("UPDATE bouys SET tzid = '{tzid}' WHERE id = '{bouy_id}';\n",tzid=tzid, bouy_id=bouy_id).as_str());
        }

        tzid_insert_stmt.push_str("COMMIT;");
        connection.execute_batch(tzid_insert_stmt.as_str()).unwrap();
        connection.load_extension_disable();
    }
}
