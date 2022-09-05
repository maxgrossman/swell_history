use actix_web::{get, App, HttpRequest, HttpResponse, HttpServer, Error as AWError};
use actix_web::http::header::ContentType;
use actix_web::web::{self, Data, Query};
use actix_files as fs;
use r2d2_sqlite::{self, SqliteConnectionManager};
use serde::Deserialize;

mod db;
use db::{Pool,Queries};

#[get("bouys/{z}/{x}/{y}.json")]
async fn bouy_tiles(req: HttpRequest, db: Data<Pool>) -> Result<HttpResponse, AWError> {
    let bouy_params: Vec<String> = vec![
        req.match_info().query("z").parse().unwrap(),
        req.match_info().query("x").parse().unwrap(),
        req.match_info().query("y").parse().unwrap()
    ];

    let features_result = db::execute(&db, Queries::GetTiles, bouy_params).await?;

    return Ok(HttpResponse::Ok()
        .insert_header(ContentType::json())
        .body(String::from(features_result.get(0).unwrap())));
}

#[derive(Debug, Deserialize)]
struct BouysRequest {
    direction: Option<String>,
    height: Option<String>,
    period: Option<String>
}

#[get("bouys/{bouy_id}")]
async fn bouy(req: HttpRequest, queryParams: Query<BouysRequest>, db: Data<Pool>) -> Result<HttpResponse, AWError> {
    let bouy_params = vec![
        req.match_info().query("bouy_id").parse().unwrap(),
        queryParams.direction.clone().unwrap_or(String::from("")),
        queryParams.height.clone().unwrap_or(String::from("")),
        queryParams.period.clone().unwrap_or(String::from(""))
    ];

    let bouys_result = db::execute(&db, Queries::GetBouyReadings, bouy_params).await?;

    return Ok(HttpResponse::Ok()
        .insert_header(ContentType::json())
        .body(String::from(bouys_result.get(0).unwrap())));    
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let manager = SqliteConnectionManager::file("bouys.sqlite")
        .with_init(|c| {
            unsafe {
                c.load_extension_enable().unwrap();
                c.load_extension("mod_spatialite", None).unwrap();
                return c.load_extension_disable();
            }
        });

    let pool = Pool::new(manager).unwrap();

    HttpServer::new(move || {
        return App::new()
            // store db pool as Data object
            .app_data(web::Data::new(pool.clone()))
            .service(fs::Files::new("/map", "./static").show_files_listing())
            .service(bouy_tiles)
            .service(bouy);
    })
    .bind(("127.0.0.1", 8008))?
    .workers(2)
    .run()
    .await
}
