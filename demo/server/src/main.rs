mod model;
#[cfg(test)]
mod test;

use {
    actix_web::{get, post, web, App, HttpRequest, HttpResponse, HttpServer},
    anyhow::Result,
    ed25519_dalek::*,
    model::*,
    mongodb::Client,
    serde::Serialize,
};

fn verify_singature<T: KeyCloner + Serialize>(req_data: &HttpRequest, req_json: &T) -> Result<()> {
    let encoded_signature = req_data
        .headers()
        .get("ed25519-singature")
        .ok_or_else(|| anyhow::Error::msg(""))?;
    let decoded_signature: &[u8] = &bs58::decode(encoded_signature).into_vec()?;
    let decoded_signature = Signature::from_bytes(decoded_signature)?;

    let decoded_pubkey: &[u8] = &bs58::decode(req_json.clone_pubkey()).into_vec()?;
    let decoded_pubkey = PublicKey::from_bytes(decoded_pubkey)?;

    let message_string = serde_json::to_string(&req_json)?;
    let message: &[u8] = message_string.as_bytes();
    decoded_pubkey.verify(message, &decoded_signature)?;
    Ok(())
}

async fn db_search_as_http<T: Contract>(
    mc: web::Data<Client>,
    pubkey: web::Path<String>,
) -> HttpResponse {
    let db = mc.default_database().expect("default db not specified");
    let pubkey = pubkey.into_inner();
    match db_search::<T>(pubkey, db).await {
        Ok(my_t) => HttpResponse::Ok().json(my_t),
        Err(SearchError::InvalidPubkey) => HttpResponse::BadRequest().body("{}"),
        Err(SearchError::NotFound) => HttpResponse::NotFound().body("{}"),
        Err(SearchError::DBError(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/swarm/{pubkey}")]
async fn get_swarm(mc: web::Data<Client>, pubkey: web::Path<String>) -> HttpResponse {
    db_search_as_http::<Swarm>(mc, pubkey).await
}

#[get("/hive/get/{pubkey}")]
async fn get_hive(mc: web::Data<Client>, pubkey: web::Path<String>) -> HttpResponse {
    db_search_as_http::<Hive>(mc, pubkey).await
}

#[get("/hive/list/top")]
async fn get_hive_top(mc: web::Data<Client>) -> HttpResponse {
    let db = mc.default_database().expect("default db not specified");
    match db_search_hive_top(db).await {
        Ok(hives) => HttpResponse::Ok().json(hives),
        Err(SearchError::InvalidPubkey) => HttpResponse::BadRequest().body("{}"),
        Err(SearchError::NotFound) => HttpResponse::NotFound().body("{}"),
        Err(SearchError::DBError(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/hive/list/neigh/{pubkey}")]
async fn get_hive_neigh(mc: web::Data<Client>, eggs: web::Path<i64>) -> HttpResponse {
    let db = mc.default_database().expect("default db not specified");
    match db_search_hive_neigh(eggs.into_inner(), db).await {
        Ok(hives) => HttpResponse::Ok().json(hives),
        Err(SearchError::InvalidPubkey) => HttpResponse::BadRequest().body("{}"),
        Err(SearchError::NotFound) => HttpResponse::NotFound().body("{}"),
        Err(SearchError::DBError(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/sacred_hive/get/{pubkey}")]
async fn get_sacred_hive(mc: web::Data<Client>, pubkey: web::Path<String>) -> HttpResponse {
    db_search_as_http::<SacredHive>(mc, pubkey).await
}

#[get("/sacred_hive/trigger/{pubkey}")]
async fn trigger_sacred_hive(mc: web::Data<Client>, pubkey: web::Path<String>) -> HttpResponse {
    match trigger(pubkey.into_inner(), mc.into_inner()).await {
        Ok(_) => HttpResponse::Ok().body("{}"),
        Err(SearchError::InvalidPubkey) => HttpResponse::BadRequest().body("{}"),
        Err(SearchError::NotFound) => HttpResponse::NotFound().body("{}"),
        Err(SearchError::DBError(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[get("/airdrop/{pubkey}")]
async fn get_airdrop(mc: web::Data<Client>, pubkey: web::Path<String>) -> HttpResponse {
    let db = mc.default_database().expect("default db not specified");
    let pubkey = pubkey.into_inner();
    match airdrop(pubkey, db).await {
        Ok(()) => HttpResponse::Ok().body("{}"),
        Err(AirdropError::InvalidPubkey) => HttpResponse::BadRequest().body("{}"),
        Err(AirdropError::AlreadyExists) => HttpResponse::Forbidden().body("{}"),
        Err(AirdropError::DBError(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[post("/hatchery")]
async fn post_hatchery(
    mc: web::Data<Client>,
    req: HttpRequest,
    item: web::Json<HatchRequest>,
) -> HttpResponse {
    let req_json = item.into_inner();
    if verify_singature(&req, &req_json).is_err() {
        return HttpResponse::Unauthorized().body("{}");
    }
    match process_hatch_request(req_json, mc.into_inner()).await {
        Ok(()) => HttpResponse::Ok().body("{}"),
        Err(StakeError::InvalidPubkey) => HttpResponse::BadRequest().body("{}"),
        Err(StakeError::NotEnoughTokens) => HttpResponse::Forbidden().body("{}"),
        Err(StakeError::DBError(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

fn parse_stake_result(r: Result<(), StakeError>) -> HttpResponse {
    match r {
        Ok(()) => HttpResponse::Ok().body("{}"),
        Err(StakeError::InvalidPubkey) => HttpResponse::BadRequest().body("{}"),
        Err(StakeError::NotEnoughTokens) => HttpResponse::Forbidden().body("{}"),
        Err(StakeError::DBError(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[post("/sacred_hive/stake")]
async fn stake_sacred_hive(
    mc: web::Data<Client>,
    req: HttpRequest,
    item: web::Json<SacredHive>,
) -> HttpResponse {
    let req_json = item.into_inner();
    if verify_singature(&req, &req_json).is_err() {
        return HttpResponse::Unauthorized().body("{}");
    }
    parse_stake_result(stake::<SacredHive>(req_json, mc.into_inner()).await)
}

#[post("/sacred_hive/unstake")]
async fn unstake_sacred_hive(
    mc: web::Data<Client>,
    req: HttpRequest,
    item: web::Json<SacredHive>,
) -> HttpResponse {
    let req_json = item.into_inner();
    if verify_singature(&req, &req_json).is_err() {
        return HttpResponse::Unauthorized().body("{}");
    }
    parse_stake_result(unstake::<SacredHive>(req_json, mc.into_inner()).await)
}

#[post("/hive/stake")]
async fn stake_hive(
    mc: web::Data<Client>,
    req: HttpRequest,
    item: web::Json<Hive>,
) -> HttpResponse {
    let req_json = item.into_inner();
    if verify_singature(&req, &req_json).is_err() {
        return HttpResponse::Unauthorized().body("{}");
    }
    parse_stake_result(stake::<Hive>(req_json, mc.into_inner()).await)
}

#[post("/hive/unstake")]
async fn unstake_hive(
    mc: web::Data<Client>,
    req: HttpRequest,
    item: web::Json<Hive>,
) -> HttpResponse {
    let req_json = item.into_inner();
    if verify_singature(&req, &req_json).is_err() {
        return HttpResponse::Unauthorized().body("{}");
    }
    parse_stake_result(unstake::<Hive>(req_json, mc.into_inner()).await)
}

#[post("/hive/attack")]
async fn post_attack(
    mc: web::Data<Client>,
    req: HttpRequest,
    item: web::Json<Attack>,
) -> HttpResponse {
    let req_json = item.into_inner();
    if verify_singature(&req, &req_json).is_err() {
        return HttpResponse::Unauthorized().body("{}");
    }
    match attack(req_json, mc.into_inner()).await {
        Ok(()) => HttpResponse::Ok().body("{}"),
        Err(AttackError::NotEnoughTokens) => HttpResponse::Forbidden().body("{}"),
        Err(AttackError::InvalidPubkey) => HttpResponse::BadRequest().body("{}"),
        Err(AttackError::NotFound) => HttpResponse::NotFound().body("{}"),
        Err(AttackError::DBError(e)) => HttpResponse::InternalServerError().body(e.to_string()),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let uri = std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".into());
    let mc = Client::with_uri_str(uri).await.expect("failed to connect");
    let db = &mc.default_database().expect("default db not specified");
    create_db_indexes(db).await;
    init_mockup_db(db).await;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(mc.clone()))
            .service(get_airdrop)
            .service(get_swarm)
            .service(get_hive)
            .service(get_hive_top)
            .service(get_hive_neigh)
            .service(get_sacred_hive)
            .service(stake_sacred_hive)
            .service(stake_hive)
            .service(unstake_sacred_hive)
            .service(unstake_hive)
            .service(post_attack)
            .service(post_hatchery)
            .service(trigger_sacred_hive)
    })
    .bind(("127.0.0.1", 9000))?
    .run()
    .await
}
