// const {get} = require('https');
// const {chain} = require('stream-chain');
// const {parser} = require('stream-json');
// const {pick} = require('stream-json/filters/Pick');
// const {streamArray} = require('stream-json/streamers/StreamArray');
// const {pointToTile} = require('@mapbox/tilebelt');
// const {find} = require('geo-tz');
// const {utilWriteFile} = require('./utils');

// module.exports = function() {
//   const zooms = [...Array(5).keys()].map(i => 16 + i),
//         tiles = {},
//         bouyHistory = {}

//   return new Promise(resolve => {
//     get('https://www.ndbc.noaa.gov/ndbcmapstations.json', (res) => {
//       const pipeline = chain([
//         res,
//         parser(),
//         pick({filter: 'station'}),
//         streamArray()
//       ])

//       pipeline.on('data', (bouy) => {
//         if (!bouyHistory[bouy.value.id]) {
//           bouyHistory[bouy.value.id] = {
//             timezone: find(bouy.value.lat, bouy.value.lon).join(','),
//             filenames: {}
//           }
//         }
//         zooms.forEach(zoom => {
//           const tileKey = pointToTile(bouy.value.lon, bouy.value.lat, zoom).reverse().join('/')
//         })
//       })

//       pipeline.on('end', () => {
//         Object.keys(tiles).forEach(tileKey =>
//           utilWriteFile(`tiles/${tileKey}.geojson`, JSON.stringify(tiles[tileKey])))

//         resolve(bouyHistory);
//       })
//     })
//   })
// }



use std::collections::HashMap;
use std::f64::consts::{PI,E};
use std::fs::File;
use serde_derive::{Serialize, Deserialize};
use serde_json::Value;

// https://github.com/mapbox/tilebelt
const D2R: f64 = PI / 180.0;
fn point_to_tile_key(lon: f64, lat: f64, z: u32) -> String{
    let sin: f64 = (lat * D2R).sin();
    let z2: f64 = f64::from(2u32.pow(z));
    let mut x: f64 = z2 * (lon / 360.0 + 0.5);
    let y: f64 = z2 * (0.5 - 0.25 * ((1.0_f64 + sin) / (1.0_f64 - sin)).log(E) / PI);
    x %= z2;
    if x < 0_f64 { x += z2 };
    return format!("{x}/{y}/{z}", x = x.floor() as u64, y = y.floor() as u64, z = z as u8);
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
    filenames: HashMap<String, bool>
}

impl BouyHistEntry {
    pub fn new(timezone: String) -> Self {
        Self {
            timezone: timezone,
            filenames: HashMap::new()
        }
    }
}

fn main() {
    let mut tiles: HashMap<String, GeoJsonFeatureCollection> = HashMap::new();
    let mut bouy_history: HashMap<String,BouyHistEntry> = HashMap::new();
    let client_result = reqwest::blocking::Client::builder().build();
    match client_result {
        Ok(client) => {
            match client.get("https://www.ndbc.noaa.gov/ndbcmapstations.json").send() {
                Ok(resp) => {
                    let bouys_json: Value = serde_json::from_str(resp.text().unwrap().as_str()).unwrap();
                    for station in bouys_json["station"].as_array().unwrap() {
                        let bouy_id: String = String::from(station["id"].as_str().unwrap());
                        if !bouy_history.contains_key(&bouy_id) {
                            // todo: use zone detect binding for timezone lookup!
                            bouy_history.insert(bouy_id, BouyHistEntry::new(String::from("foo")));
                        }
                        for zoom in 16..17 {
                            let lon = station["lon"].as_f64().unwrap();
                            let lat = station["lat"].as_f64().unwrap();
                            let tile_key: String = point_to_tile_key(lon, lat, zoom);
                            if !tiles.contains_key(&tile_key) {
                                tiles.insert(tile_key.clone(), GeoJsonFeatureCollection::new());
                            }

                            tiles
                                .get_mut(&tile_key)
                                .unwrap()
                                .add_feature(GeoJsonPointFeature::new([lon,lat]));
                        }
                    }
                }
                Err(err) => {
                    println!("Request Error: {}", err);
                }
            }
        },
        Err(err) => {
            println!("Error: {}", err);
        }
    }

    match serde_json::to_writer(&File::create("tiles.json").unwrap(), &tiles) {
        Ok(written) => {}
        Err(err) => {}
    }
    match serde_json::to_writer(&File::create("bouy_history.json").unwrap(), &bouy_history) {
        Ok(written) => {}
        Err(err) => {}
    }
}
