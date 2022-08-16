use std::env::args;
use std::collections::HashMap;
use std::f64::consts::{PI,E};
use std::fs::File;
use serde_derive::{Serialize, Deserialize};
use serde_json::{Value};
use rusqlite::{Connection, NO_PARAMS};


// https://github.com/mapbox/tilebelt
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

#[derive(Serialize, Deserialize)]
struct GeoJsonPoint {
    r#type: String,
    coordinates: [f64; 2]
}

impl GeoJsonPoint {
    pub fn new(coordinates: [f64;2]) -> Self {
        Self {
            r#type: String::from("Point"),
            coordinates: coordinates
        }
    }
}

#[derive(Serialize, Deserialize)]
struct GeoJsonPointFeature {
    r#type: String,
    properties: HashMap<String,String>,
    geometry: GeoJsonPoint
}

impl GeoJsonPointFeature {
    pub fn new(coordinates: [f64;2]) -> Self {
        Self {
            r#type: String::from("Feature"),
            properties: HashMap::new(),
            geometry: GeoJsonPoint::new(coordinates)
        }
    }
}

#[derive(Serialize, Deserialize)]
struct GeoJsonFeatureCollection {
    r#type: String,
    features: Vec<GeoJsonPointFeature>
}

impl GeoJsonFeatureCollection {
    pub fn new() -> Self {
        return Self { 
            r#type: String::from("FeatureCollection"),
            features: Vec::new()
        }
    }
    pub fn add_feature(&mut self, feature: GeoJsonPointFeature) {
        self.features.push(feature);
    }
}


#[derive(Serialize, Deserialize)]

struct BouyHistEntry {
    timezone: String,
    loc: [f64;2],
    filenames: HashMap<String, bool>
}

impl BouyHistEntry {
    pub fn new(timezone: String, lon: f64, lat: f64) -> Self {
        Self {
            timezone: timezone,
            loc: [lon,lat],
            filenames: HashMap::new()
        }
    }
}

fn main() {
    let timezones_db: String = args().collect::<Vec<String>>().pop().unwrap();
    match Connection::open("bouys.sqlite") {
        Ok(mut connection) => {
            let mut tiles: HashMap<String, GeoJsonFeatureCollection> = HashMap::new();
            let mut bouy_history: HashMap<String,BouyHistEntry> = HashMap::new();
            let client_result = reqwest::blocking::Client::builder().build();
            match client_result {
                Ok(client) => {
                    match client.get("https://www.ndbc.noaa.gov/ndbcmapstations.json").send() {
                        Ok(resp) => {
                            let bouys_json: Value = serde_json::from_str(resp.text().unwrap().as_str()).unwrap();
                            match client.get("https://www.ndbc.noaa.gov/data/historical/stdmet/").send() {
                                Ok(history_resp) => {
                                    unsafe {
                                        connection.load_extension_enable();
                                        let r = connection.load_extension("mod_spatialite", None);
                                        connection.execute(format!("ATTACH \"{db}\" AS tzdb;",db=timezones_db).as_str(), NO_PARAMS).unwrap();
                                        let mut bouys_statement = String::from(
                                            "
                                            BEGIN TRANSACTION;
                                            CREATE TABLE bouy_tile_indexes(bouy_id text, zoom integer, x integer, y integer);
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
        
                                            for zoom in 16..17 {
                                                let tile_x_y: [u64;2] = point_to_tile_x_y(lon, lat, zoom);
                                                bouys_statement.push_str(format!(
                                                    "\nINSERT INTO bouy_tile_indexes values ('{bouy_id}', {zoom}, {x}, {y});",
                                                    bouy_id=bouy_id, zoom=zoom, x=tile_x_y[0], y=tile_x_y[1]
                                                ).as_str());
                                            }
                                    
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
                                },
                                Err(err) => {}
                            }
                        },
                        Err(err) => {
                            println!("Request Error: {}", err);
                        }
                    }
                },
                Err(err) => {
                    println!("Error: {}", err);
                }
            }
        },
        Err(err) => { panic!("{}", err); }
    }
}
