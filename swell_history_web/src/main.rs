use actix_web::{get, App, HttpRequest, HttpResponse, HttpServer, Responder};
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_files as fs;
use std::sync::Mutex;
use rusqlite::{Connection};


//readings like a time
//readings around today.

#[get("bouys/{z}/{x}/{y}.json")]
async fn bouy_tiles(req: HttpRequest) -> impl Responder {
    let connection = req.app_data::<Data<Mutex<Connection>>>().unwrap().lock().unwrap();
    let z: u32 = req.match_info().query("z").parse().unwrap();
    let x: u32 = req.match_info().query("x").parse().unwrap();
    let y: u32 = req.match_info().query("y").parse().unwrap();
    let mut stmt = connection.prepare(format!(
        "
            SELECT
                json_object(
                    'type', 'FeatureCollection',
                    'features', json_group_array(
                        json_object(
                            'properties', json_object('bouy_id', bouy_tile_indexes.bouy_id),
                            'geometry', json(AsGeoJSON(bouys.geometry))
                        )
                    )
                )
            FROM bouy_tile_indexes join bouys on bouys.id = bouy_tile_indexes.bouy_id
            WHERE zoom = {z} and x = {x} and y = {y};
        ",
        z = z, x = x, y = y
    ).as_str()).unwrap();

    let bouys_feature_collection: String = stmt.query(()).unwrap()
        .next().unwrap()
        .unwrap().get(0).unwrap();

    return HttpResponse::Ok()
        .insert_header(ContentType::json())
        .body(bouys_feature_collection);
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        let connection = Connection::open( String::from("bouys.sqlite")).unwrap();
        unsafe {
            connection.load_extension_enable();
            let r = connection.load_extension("mod_spatialite", None);
            connection.load_extension_disable();
        }
        let database_connection: Data<Mutex<Connection>> = Data::new(Mutex::new(connection));

        App::new()
            .app_data(Data::clone(&database_connection))
            .service(fs::Files::new("/map", "./static").show_files_listing())
            .service(bouy_tiles)

    })
    .bind(("127.0.0.1",8008))?
    .run()
    .await
}
