
use actix_web::{error, web, Error};
type BouyDbStatementAgg = Result<Vec<String>, rusqlite::Error>;

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

pub enum Queries {
    GetTiles,
    GetBouyReadings
}

const SIGNATURE_LOOKUP: [char;3] = ['d','h','P'];

fn get_bouy_readings (conn: Connection, params: Vec<String>) -> BouyDbStatementAgg {
    let mut filtered_timestamps = format!(
        "
        select reading_time, d , h / 100.0 as h, p / 100.0 as p
        from timestamps_{bouy_id}
        ",
        bouy_id = params.get(0).unwrap()
    );


    let mut filter_clause = String::from("WHERE");

    for i in 1..params.len() - 1 {
        let param = params.get(i).unwrap();
        if !param.is_empty() {
            let bounds: Vec<&str> = param.split(",").collect();
            match bounds.len() {
                1 => { filter_clause.push_str(format!("\n{sig} = {param}", sig=SIGNATURE_LOOKUP[i], param=bounds.get(0).unwrap() ).as_str()) },
                2 => { filter_clause.push_str(format!("\n{sig} >= {min} and {sig} <= {max}", sig=SIGNATURE_LOOKUP[i], min=bounds.get(0).unwrap(),  max=bounds.get(0).unwrap()).as_str());},
                _ => {}
            }
        }
    }

    if filter_clause.ne(&String::from("WHERE")) {
        filtered_timestamps.push_str(&filter_clause.as_str());
    }

    let offset: usize = params.last().unwrap().to_string().parse::<usize>().unwrap();
    filtered_timestamps.push_str(format!("\nLIMIT {LIMIT}\nOFFSET {OFFSET}", LIMIT=10, OFFSET=offset).as_str());

    let query = format!(
        "
        with fitered_timestamps as ({filtered_timestamps})
        SELECT json_group_array(
            json_object(
                'reading_time', reading_time,
                'direction', d,
                'period', p,
                'height', h
            )
        )
        FROM fitered_timestamps;
        ",
        filtered_timestamps = filtered_timestamps
    );

    return conn.prepare(query.as_str()).unwrap().query_map([], |row| {
        Ok(row.get(0)?)
    })
    .and_then(Iterator::collect);
}

fn get_tiles (conn: Connection, params: Vec<String>) -> BouyDbStatementAgg {
    let mut stmt = conn.prepare(format!(
        "
            SELECT json_group_array(
                json_object(
                    'type', 'Feature',
                    'properties', json_object('id', bouy_tile_indexes.bouy_id),
                    'geometry', json(AsGeoJSON(bouys.geometry)
                ))
            )
            FROM bouy_tile_indexes join bouys on bouys.id = bouy_tile_indexes.bouy_id
            WHERE zoom = {z} and x = {x} and y = {y};
        ",
        z = params.get(0).unwrap(), x = params.get(1).unwrap(), y = params.get(2).unwrap()
    ).as_str()).unwrap();

    return stmt.query_map([], |row| {
        Ok(row.get(0)?)
    })
    .and_then(Iterator::collect);
}

pub async fn execute(pool: &Pool, query: Queries, params: Vec<String>) -> Result<Vec<String> ,Error> {
    let pool = pool.clone();
    let conn = web::block(move || pool.get())
        .await?
        .map_err(error::ErrorInternalServerError)?;

    web::block(move || {
        match query {
            Queries::GetTiles => get_tiles(conn, params),
            Queries::GetBouyReadings => get_bouy_readings(conn, params)
        }
    })
    .await?
    .map_err(error::ErrorInternalServerError)
}
