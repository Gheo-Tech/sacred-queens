#![cfg(test)]

use {
    super::{model::*, *},
    actix_http::{body::MessageBody, Request},
    actix_web::{
        dev::{Service, ServiceResponse},
        error::Error,
        rt::time::timeout,
        test::{call_service, init_service, read_body, TestRequest},
    },
    ed25519_dalek::*,
    http::StatusCode,
    mongodb::{bson::doc, Client, Database},
    serde::Serialize,
    std::time::Duration,
};

#[derive(Debug)]
enum TestMethod {
    Post,
    Get,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct Empty {}

struct TestData<'a, Req: Serialize, Res: Serialize> {
    method: TestMethod,
    uri: String,
    keypair: &'a Keypair,
    req: Req,
    res: Res,
    status: StatusCode,
}

impl<'a, Req: Serialize, Res: Serialize> TestData<'a, Req, Res> {
    async fn run<S, B>(self, app: &S)
    where
        S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
        B: MessageBody,
    {
        let req = match self.method {
            TestMethod::Post => {
                let message_string = serde_json::to_string(&self.req).unwrap();
                let message: &[u8] = message_string.as_bytes();
                let signature: Signature = self.keypair.sign(message);
                let encoded_signature = bs58::encode(signature).into_string();
                TestRequest::post()
                    .uri(&self.uri)
                    .set_json(&self.req)
                    .insert_header(("ed25519-singature", encoded_signature))
                    .to_request()
            }
            TestMethod::Get => TestRequest::get().uri(&self.uri).to_request(),
        };
        println!("testing... {:?} {}", self.method, self.uri);
        let response = timeout(Duration::from_secs(2), call_service(&app, req))
            .await
            .unwrap();
        assert_eq!(self.status, response.status());
        let response_body = timeout(Duration::from_secs(2), read_body(response))
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_string(&self.res).expect("could not serlialize json"),
            response_body
        );
    }
}

macro_rules! perform_test {
    ($a:expr, $k:expr, $uri:expr, $status:expr) => {
        TestData {
            method: TestMethod::Get,
            uri: $uri,
            keypair: $k,
            req: Empty {},
            res: Empty {},
            status: $status,
        }
        .run($a)
        .await;
    };

    ($a:expr, $k:expr, $uri:expr, $res: expr, $status:expr) => {
        TestData {
            method: TestMethod::Get,
            uri: $uri,
            keypair: $k,
            req: Empty {},
            res: $res,
            status: $status,
        }
        .run($a)
        .await;
    };

    ($a:expr, $k:expr, $uri:expr, $req: expr, $res: expr, $status:expr) => {
        TestData {
            method: TestMethod::Post,
            uri: $uri,
            keypair: $k,
            req: $req,
            res: $res,
            status: $status,
        }
        .run($a)
        .await;
    };
}

macro_rules! db_insert {
    ($db:expr, $coll:expr, $object:expr) => {
        $db.collection($coll)
            .insert_one($object, None)
            .await
            .unwrap();
    };
}

macro_rules! init_app_and_db {
    ($($service:expr),*) => {{
        let uri = std::env::var("MONGODB_URI").unwrap();
        let mongo_client = Client::with_uri_str(uri)
            .await
            .expect("failed to connect to database");
        let app = init_service(
            App::new()
                .app_data(web::Data::new(mongo_client.clone()))
                $(.service($service))*,
        )
        .await;
        let db = mongo_client
            .default_database()
            .expect("default db not specified");
        (app, db)
    }};
}

#[actix_web::test]
async fn airdrop_and_swarm() {
    let (app, _) = init_app_and_db!(get_airdrop, get_swarm);
    let keypair = generate_keypair();
    let pubkey = get_pubkey(&keypair);
    macro_rules! wrap_test {
        ($($param:expr),*) => {
            perform_test!(&app, &keypair $(,$param)*);
        };
    }

    // valid airdrop request
    wrap_test!("/airdrop/".to_string() + &pubkey, StatusCode::OK);

    // 2nd airdrop reuqest with the same pubkey - should fail
    wrap_test!("/airdrop/".to_string() + &pubkey, StatusCode::FORBIDDEN);

    // get account to see it has 10 sacred_queens
    wrap_test!(
        "/swarm/".to_string() + &pubkey,
        Swarm {
            berserkers: 0,
            pubkey: pubkey.clone(),
            sacred_queens: 10,
            queens: 0,
            guardians: 0,
            eggs: 0,
        },
        StatusCode::OK
    );

    // invalid public key on get swarm
    wrap_test!(
        "/airdrop/thisIsABadString".to_string(),
        StatusCode::BAD_REQUEST
    );

    // get swarm with random pubkey that does not exist
    wrap_test!(
        "/swarm/CF4eGJXudCwqnEgTyhQ6LwsrkqE3myoEoen6rYzVFwif".to_string(),
        StatusCode::NOT_FOUND
    );
}

#[actix_web::test]
async fn hatch() {
    let (app, db) = init_app_and_db!(post_hatchery);

    let keypair = generate_keypair();
    let pubkey = get_pubkey(&keypair);

    macro_rules! wrap_test {
        ($($param:expr),*) => {
            perform_test!(&app, &keypair $(,$param)*);
        };
    }

    db_insert!(
        db,
        SWARMS_COLL_NAME,
        Swarm {
            berserkers: 0,
            pubkey: pubkey.clone(),
            sacred_queens: 0,
            queens: 0,
            guardians: 0,
            eggs: 10000,
        }
    );

    // try to hatch more eggs than in account - should fail
    wrap_test!(
        "/hatchery".to_string(),
        HatchRequest {
            pubkey: pubkey.clone(),
            eggs: 10001,
        },
        Empty {},
        StatusCode::FORBIDDEN
    );

    // hatch eggs
    wrap_test!(
        "/hatchery".to_string(),
        HatchRequest {
            pubkey: pubkey.clone(),
            eggs: 10000,
        },
        Empty {},
        StatusCode::OK
    );

    let swarm = match db_search::<Swarm>(pubkey.clone(), db.clone()).await {
        Ok(s) => s,
        Err(_) => panic!("Failed to get swarm {}", pubkey),
    };

    if swarm.eggs > 0
        || swarm.queens > 120
        || swarm.guardians > 1100
        || swarm.berserkers < 8800
        || swarm.queens + swarm.guardians + swarm.berserkers != 10000
    {
        panic!(
            "Swarm does not have correct ammount of tokens: \n{}",
            serde_json::to_string(&swarm).unwrap()
        );
    }
}

#[actix_web::test]
async fn sacred_hive() {
    let (app, db) = init_app_and_db!(
        get_swarm,
        get_sacred_hive,
        stake_sacred_hive,
        unstake_sacred_hive,
        trigger_sacred_hive
    );
    let keypair = generate_keypair();
    let pubkey = get_pubkey(&keypair);

    macro_rules! wrap_test {
        ($($param:expr),*) => {
            perform_test!(&app, &keypair $(,$param)*);
        };
    }

    // create new_swarm that will be used for testing purposes
    db_insert!(
        db,
        SWARMS_COLL_NAME,
        Swarm {
            berserkers: 300,
            pubkey: pubkey.clone(),
            sacred_queens: 200,
            queens: 50,
            guardians: 0,
            eggs: 100,
        }
    );
    db_insert!(
        db,
        SACRED_HIVE_COLL_NAME,
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 0,
            eggs: 0,
        }
    );

    // stake first batch of tokens - should succeed
    wrap_test!(
        "/sacred_hive/stake".to_string(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 50,
            eggs: 100,
        },
        Empty {},
        StatusCode::OK
    );

    // get staked data
    wrap_test!(
        "/sacred_hive/get/".to_string() + &pubkey.clone(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 50,
            eggs: 100,
        },
        StatusCode::OK
    );

    // stake 2nd batch of tokens - should succeed
    wrap_test!(
        "/sacred_hive/stake".to_string(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 150,
            eggs: 0,
        },
        Empty {},
        StatusCode::OK
    );

    // get staked data
    wrap_test!(
        "/sacred_hive/get/".to_string() + &pubkey.clone(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 200,
            eggs: 100,
        },
        StatusCode::OK
    );

    // unstake tokens - should succeed
    wrap_test!(
        "/sacred_hive/unstake".to_string(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 0,
            eggs: 100,
        },
        Empty {},
        StatusCode::OK
    );

    // get swarm data after staking and unstaking
    wrap_test!(
        "/swarm/".to_string() + &pubkey.clone(),
        Swarm {
            pubkey: pubkey.clone(),
            berserkers: 300,
            guardians: 0,
            sacred_queens: 0,
            queens: 50,
            eggs: 100,
        },
        StatusCode::OK
    );

    // get hive after staking and unstaking
    wrap_test!(
        "/sacred_hive/get/".to_string() + &pubkey.clone(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 200,
            eggs: 0,
        },
        StatusCode::OK
    );

    // trigger all sacred queens to lay eggs
    wrap_test!(
        "/sacred_hive/trigger/".to_string() + &pubkey.clone(),
        StatusCode::OK
    );

    // get hive after laying eggs
    wrap_test!(
        "/sacred_hive/get/".to_string() + &pubkey.clone(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 200,
            eggs: 20000,
        },
        StatusCode::OK
    );

    // try to stake more tokens than in swarm - should fail
    wrap_test!(
        "/sacred_hive/stake".to_string(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 500,
            eggs: 0,
        },
        Empty {},
        StatusCode::FORBIDDEN
    );

    // try to unstake more tokens than in hive - should fail
    wrap_test!(
        "/sacred_hive/unstake".to_string(),
        SacredHive {
            pubkey: pubkey.clone(),
            sacred_queens: 0,
            eggs: 9999999,
        },
        Empty {},
        StatusCode::FORBIDDEN
    );
}

#[actix_web::test]
async fn hive() {
    let (app, db) = init_app_and_db!(get_hive, stake_hive, unstake_hive, get_swarm);

    let keypair = generate_keypair();
    let pubkey = get_pubkey(&keypair);

    macro_rules! wrap_test {
        ($($param:expr),*) => {
            perform_test!(&app, &keypair $(,$param)*);
        };
    }

    // create new_swarm that will be used for testing purposes
    db_insert!(
        db,
        SWARMS_COLL_NAME,
        Swarm {
            berserkers: 0,
            pubkey: pubkey.clone(),
            sacred_queens: 0,
            queens: 5,
            guardians: 170,
            eggs: 0,
        }
    );

    // add some existing tokens to the sacred_hive of this swarm
    db_insert!(
        db,
        HIVE_COLL_NAME,
        Hive {
            pubkey: pubkey.clone(),
            guardians: 150,
            queens: 20,
            eggs: 100,
        }
    );

    // stake tokens - should succeed
    wrap_test!(
        "/hive/stake".to_string(),
        Hive {
            pubkey: pubkey.clone(),
            queens: 5,
            guardians: 70,
            eggs: 0,
        },
        Empty {},
        StatusCode::OK
    );

    // get staked data
    wrap_test!(
        "/hive/get/".to_string() + &pubkey.clone(),
        Hive {
            pubkey: pubkey.clone(),
            guardians: 220,
            queens: 25,
            eggs: 100,
        },
        StatusCode::OK
    );

    // unstake tokens - should succeed
    wrap_test!(
        "/hive/unstake".to_string(),
        Hive {
            pubkey: pubkey.clone(),
            guardians: 0,
            queens: 0,
            eggs: 100,
        },
        Empty {},
        StatusCode::OK
    );

    // get swarm data after staking and unstaking
    wrap_test!(
        "/swarm/".to_string() + &pubkey.clone(),
        Swarm {
            pubkey: pubkey.clone(),
            berserkers: 0,
            guardians: 100,
            sacred_queens: 0,
            queens: 0,
            eggs: 100,
        },
        StatusCode::OK
    );

    // try to stake more tokens than in swarm - should fail
    wrap_test!(
        "/hive/stake".to_string(),
        Hive {
            pubkey: pubkey.clone(),
            guardians: 2500,
            queens: 500,
            eggs: 0,
        },
        Empty {},
        StatusCode::FORBIDDEN
    );

    // try to unstake more tokens than in hive - should fail
    wrap_test!(
        "/hive/unstake".to_string(),
        Hive {
            pubkey: pubkey.clone(),
            guardians: 0,
            queens: 800,
            eggs: 2000,
        },
        Empty {},
        StatusCode::FORBIDDEN
    );
}

#[actix_web::test]
async fn hive_list() {
    let (app, db) = init_app_and_db!(get_hive_top, get_hive_neigh);
    let coll = db.collection(HIVE_COLL_NAME);

    let mut hives: Vec<Hive> = Vec::new();
    for i in 0..50 {
        hives.push(Hive {
            pubkey: get_pubkey(&generate_keypair()),
            guardians: 100,
            queens: 10,
            eggs: 5000 + i,
        });
    }

    coll.delete_many(doc! { "eggs": { "$gte": 5000} }, None)
        .await
        .unwrap();
    coll.insert_many(hives.clone(), None).await.unwrap();

    // get top ten hives
    let mut top_ten: Vec<Hive> = hives.clone().drain(40..).collect();
    top_ten.sort_by(|a, b| b.eggs.cmp(&a.eggs));
    TestData {
        method: TestMethod::Get,
        uri: "/hive/list/top".to_string(),
        keypair: &generate_keypair(),
        req: Empty {},
        res: top_ten,
        status: StatusCode::OK,
    }
    .run(&app)
    .await;

    // get 10 hives that have about 5015 eggs
    let mut neighbours: Vec<Hive> = hives
        .clone()
        .drain(11..)
        .collect::<Vec<Hive>>()
        .drain(..10)
        .collect();
    neighbours.sort_by(|a, b| b.eggs.cmp(&a.eggs));
    TestData {
        method: TestMethod::Get,
        uri: "/hive/list/neigh/5015".to_string(),
        keypair: &generate_keypair(),
        req: Empty {},
        res: neighbours,
        status: StatusCode::OK,
    }
    .run(&app)
    .await;
}

#[actix_web::test]
async fn attack_errors() {
    let (app, db) = init_app_and_db!(post_attack);
    let attacker_keypair = generate_keypair();
    let attacker_pubkey = get_pubkey(&attacker_keypair);
    let defender_keypair = generate_keypair();
    let defender_pubkey = get_pubkey(&defender_keypair);
    macro_rules! wrap_test {
        ($($param:expr),*) => {
            perform_test!(&app, &attacker_keypair $(,$param)*);
        };
    }

    db_insert!(
        db,
        SWARMS_COLL_NAME,
        Swarm {
            pubkey: attacker_pubkey.clone(),
            berserkers: 900,
            eggs: 0,
            queens: 0,
            sacred_queens: 0,
            guardians: 0,
        }
    );

    db_insert!(
        db,
        HIVE_COLL_NAME,
        Hive {
            pubkey: defender_pubkey.clone(),
            queens: 10,
            guardians: 90,
            eggs: 100,
        }
    );

    // test attack with an invalid swarm pubkey - should fail
    wrap_test!(
        "/hive/attack".to_string(),
        Attack {
            swarm_pubkey: "thisIsABadString".to_string(),
            hive_pubkey: defender_pubkey.clone(),
            berserkers: 100,
        },
        Empty {},
        StatusCode::UNAUTHORIZED
    );

    // test attack with an invalid hive pubkey - should fail
    wrap_test!(
        "/hive/attack".to_string(),
        Attack {
            swarm_pubkey: attacker_pubkey.clone(),
            hive_pubkey: "thisIsABadString".to_string(),
            berserkers: 100,
        },
        Empty {},
        StatusCode::BAD_REQUEST
    );

    // try to attack without having enough berserkers - should fail
    wrap_test!(
        "/hive/attack".to_string(),
        Attack {
            swarm_pubkey: attacker_pubkey,
            hive_pubkey: defender_pubkey.clone(),
            berserkers: 99999,
        },
        Empty {},
        StatusCode::FORBIDDEN
    );

    // try to attack without having an account - should fail
    let random_keypair = generate_keypair();
    let random_pubkey = get_pubkey(&random_keypair);
    perform_test!(
        &app,
        &random_keypair,
        "/hive/attack".to_string(),
        Attack {
            swarm_pubkey: random_pubkey,
            hive_pubkey: defender_pubkey.clone(),
            berserkers: 100,
        },
        Empty {},
        StatusCode::FORBIDDEN
    );
}

#[actix_web::test]
async fn attack_balance() {
    let test_count: usize = std::env::var("ATTACK_TEST_COUNT")
        .unwrap()
        .parse::<usize>()
        .unwrap();

    let (app, db) = init_app_and_db!(post_attack);
    let app = std::rc::Rc::new(app);
    let handles = (0..test_count)
        .map(|_| {
            let app = app.clone();
            let db = db.clone();
            actix_web::rt::spawn(async { perform_attack(app, db).await })
        })
        .collect::<Vec<_>>();

    let mut attackers_eggs = 0;
    let mut defenders_eggs = 0;
    for handle in handles {
        let result = handle.await.unwrap();
        attackers_eggs += result.0;
        defenders_eggs += result.1;
    }

    if attackers_eggs < test_count as i64 * 40
        || attackers_eggs > test_count as i64 * 50
        || defenders_eggs < test_count as i64 * 50
        || defenders_eggs > test_count as i64 * 60
    {
        panic!(
            "Resource balance after attack failed.
             Total attacks: {}
             attackers eggs: {}
             defenders eggs: {}",
            test_count, attackers_eggs, defenders_eggs
        );
    }
}

async fn perform_attack<S, B>(app: S, db: Database) -> (i64, i64)
where
    S: Service<Request, Response = ServiceResponse<B>, Error = Error>,
    B: MessageBody,
{
    let attacker_keypair = generate_keypair();
    let attacker_pubkey = get_pubkey(&attacker_keypair);
    let defender_keypair = generate_keypair();
    let defender_pubkey = get_pubkey(&defender_keypair);

    db_insert!(
        db,
        SWARMS_COLL_NAME,
        Swarm {
            pubkey: attacker_pubkey.clone(),
            berserkers: 900,
            eggs: 0,
            queens: 0,
            sacred_queens: 0,
            guardians: 0,
        }
    );

    db_insert!(
        db,
        HIVE_COLL_NAME,
        Hive {
            pubkey: defender_pubkey.clone(),
            queens: 10,
            guardians: 90,
            eggs: 100,
        }
    );

    perform_test!(
        &app,
        &attacker_keypair,
        "/hive/attack".to_string(),
        Attack {
            swarm_pubkey: attacker_pubkey.clone(),
            hive_pubkey: defender_pubkey.clone(),
            berserkers: 900,
        },
        Empty {},
        StatusCode::OK
    );

    let swarm_eggs = match db_search::<Swarm>(attacker_pubkey.clone(), db.clone()).await {
        Ok(s) => s.eggs,
        Err(_) => panic!("Failed to get swarm {}", attacker_pubkey),
    };

    let hive_eggs = match db_search::<Hive>(defender_pubkey.clone(), db.clone()).await {
        Ok(h) => h.eggs,
        Err(_) => panic!("Failed to get hive {}", defender_pubkey),
    };

    (swarm_eggs, hive_eggs)
}

#[actix_web::test]
async fn unauthorized_requests() {
    let (app, db) = init_app_and_db!(
        get_hive,
        stake_hive,
        stake_sacred_hive,
        unstake_hive,
        unstake_sacred_hive,
        post_attack
    );

    let real_keypair = generate_keypair();
    let real_pubkey = get_pubkey(&real_keypair);
    let fake_keypair = generate_keypair();
    let fake_pubkey = get_pubkey(&fake_keypair);

    // create new_swarm that will be used for testing purposes
    db_insert!(
        db,
        SWARMS_COLL_NAME,
        Swarm {
            berserkers: 0,
            pubkey: real_pubkey.clone(),
            sacred_queens: 0,
            queens: 5,
            guardians: 170,
            eggs: 0,
        }
    );

    // add some existing tokens to the hive of this swarm
    db_insert!(
        db,
        HIVE_COLL_NAME,
        Hive {
            pubkey: real_pubkey.clone(),
            guardians: 150,
            queens: 20,
            eggs: 100,
        }
    );

    // add some existing tokens to the sacred_hive of this swarm
    db_insert!(
        db,
        SACRED_HIVE_COLL_NAME,
        SacredHive {
            pubkey: real_pubkey.clone(),
            sacred_queens: 0,
            eggs: 0,
        }
    );

    // unauthorized sacred hive stake post
    perform_test!(
        &app,
        &fake_keypair,
        "/sacred_hive/stake".to_string(),
        SacredHive {
            pubkey: real_pubkey.clone(),
            sacred_queens: 50,
            eggs: 100,
        },
        Empty {},
        StatusCode::UNAUTHORIZED
    );

    // unauthorized hive stake post
    perform_test!(
        &app,
        &fake_keypair,
        "/hive/stake".to_string(),
        Hive {
            pubkey: real_pubkey.clone(),
            queens: 5,
            guardians: 70,
            eggs: 0,
        },
        Empty {},
        StatusCode::UNAUTHORIZED
    );

    // unauthorized attack
    perform_test!(
        &app,
        &fake_keypair,
        "/hive/attack".to_string(),
        Attack {
            swarm_pubkey: real_pubkey.clone(),
            hive_pubkey: fake_pubkey.clone(),
            berserkers: 900,
        },
        Empty {},
        StatusCode::UNAUTHORIZED
    );
}

#[actix_web::test]
#[ignore = "run with '-- --ignored' to clean the DB"]
async fn clean_db() {
    let uri = std::env::var("MONGODB_URI").unwrap();
    let db = Client::with_uri_str(uri)
        .await
        .expect("failed to connect")
        .default_database()
        .expect("default db not specified");

    // Clear any data currently in the users collection.
    db.collection::<Swarm>(SWARMS_COLL_NAME)
        .drop(None)
        .await
        .expect("drop collection should succeed");

    db.collection::<SacredHive>(SACRED_HIVE_COLL_NAME)
        .drop(None)
        .await
        .expect("drop collection should succeed");

    db.collection::<Hive>(HIVE_COLL_NAME)
        .drop(None)
        .await
        .expect("drop collection should succeed");
}
