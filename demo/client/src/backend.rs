use ed25519_dalek::*;
use reqwasm::http::Request;
use serde::{Deserialize, Serialize};

const BACKEND: &str = "http://localhost:8080/backend";

#[derive(Clone, Deserialize)]
pub struct Swarm {
    pub pubkey: String,
    pub sacred_queens: i64,
    pub queens: i64,
    pub guardians: i64,
    pub berserkers: i64,
    pub eggs: i64,
}

impl Swarm {
    pub fn new() -> Self {
        Swarm {
            pubkey: String::new(),
            sacred_queens: 0,
            queens: 0,
            guardians: 0,
            berserkers: 0,
            eggs: 0,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SacredHive {
    pub pubkey: String,
    pub sacred_queens: i64,
    pub eggs: i64,
}

#[derive(Deserialize, Clone, Serialize, PartialEq)]
pub struct Hive {
    pub pubkey: String,
    pub guardians: i64,
    pub queens: i64,
    pub eggs: i64,
}

#[derive(Deserialize, Serialize)]
pub struct HatchRequest {
    pub pubkey: String,
    pub eggs: i64,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Attack {
    pub swarm_pubkey: String,
    pub hive_pubkey: String,
    pub berserkers: i64,
}

#[derive(Clone)]
pub struct Account {
    pub swarm: Swarm,
    pub sacred_hive: SacredHive,
    pub hive: Hive,
}

impl Account {
    pub fn can_hatch(&self) -> bool {
        self.swarm.eggs.is_positive()
    }
    pub fn can_stake_h(&self) -> bool {
        self.swarm.eggs.is_positive()
            || self.swarm.queens.is_positive()
            || self.swarm.guardians.is_positive()
    }
    pub fn can_stake_s(&self) -> bool {
        self.swarm.sacred_queens.is_positive()
    }
    pub fn can_unstake_h(&self) -> bool {
        self.hive.guardians.is_positive()
            || self.hive.queens.is_positive()
            || self.hive.eggs.is_positive()
    }
    pub fn can_unstake_s(&self) -> bool {
        self.sacred_hive.sacred_queens.is_positive() || self.sacred_hive.eggs.is_positive()
    }
    pub fn can_attack(&self) -> bool {
        self.swarm.berserkers.is_positive()
    }
    pub fn new() -> Self {
        Self {
            swarm: Swarm {
                pubkey: String::new(),
                sacred_queens: 0,
                queens: 0,
                guardians: 0,
                berserkers: 0,
                eggs: 0,
            },
            hive: Hive {
                pubkey: String::new(),
                guardians: 0,
                queens: 0,
                eggs: 0,
            },
            sacred_hive: SacredHive {
                pubkey: String::new(),
                sacred_queens: 0,
                eggs: 0,
            },
        }
    }
}

pub async fn fetch_hives(eggs: i64) -> Result<Vec<Hive>, reqwasm::Error> {
    let mut url = format!("{}/hive/list/top", BACKEND);
    if eggs > 0 {
        url = format!("{}/hive/list/neigh/{}", BACKEND, eggs);
    }
    let resp = Request::get(&url).send().await?;
    let body = resp.json::<Vec<Hive>>().await?;
    Ok(body)
}

pub struct AirdropResult(pub Result<bool, reqwasm::Error>);
pub async fn get_airdrop(pubkey: String) -> Result<bool, reqwasm::Error> {
    let url = format!("{}/airdrop/{}", BACKEND, pubkey);
    let resp = Request::get(&url).send().await?;
    Ok(resp.ok())
}

pub async fn get_trigger(pubkey: String) {
    let url = format!("{}/sacred_hive/trigger/{}", BACKEND, pubkey);
    let _resp = Request::get(&url).send().await;
}

pub async fn get_swarm(pubkey: String) -> Result<Swarm, reqwasm::Error> {
    let url = format!("{}/swarm/{}", BACKEND, pubkey);
    let resp = Request::get(&url).send().await?;
    let body = resp.json::<Swarm>().await?;
    Ok(body)
}

pub async fn get_account(pubkey: String) -> Result<Account, reqwasm::Error> {
    let swarm = Request::get(&format!("{}/swarm/{}", BACKEND, pubkey))
        .send()
        .await?
        .json::<Swarm>()
        .await?;
    let sacred_hive = Request::get(&format!("{}/sacred_hive/get/{}", BACKEND, pubkey))
        .send()
        .await?
        .json::<SacredHive>()
        .await?;
    let hive = Request::get(&format!("{}/hive/get/{}", BACKEND, pubkey))
        .send()
        .await?
        .json::<Hive>()
        .await?;
    Ok(Account {
        swarm,
        sacred_hive,
        hive,
    })
}

pub struct StakeResult(pub Result<bool, reqwasm::Error>);
pub async fn stake_sacred_hive(sh: SacredHive, kp: Keypair) -> Result<bool, reqwasm::Error> {
    run_request(sh, kp, "sacred_hive/stake".to_string()).await
}
pub async fn unstake_sacred_hive(sh: SacredHive, kp: Keypair) -> Result<bool, reqwasm::Error> {
    run_request(sh, kp, "sacred_hive/unstake".to_string()).await
}
pub async fn stake_hive(h: Hive, kp: Keypair) -> Result<bool, reqwasm::Error> {
    run_request(h, kp, "hive/stake".to_string()).await
}
pub async fn unstake_hive(h: Hive, kp: Keypair) -> Result<bool, reqwasm::Error> {
    run_request(h, kp, "hive/unstake".to_string()).await
}

pub async fn hatch(hr: HatchRequest, kp: Keypair) -> Result<bool, reqwasm::Error> {
    run_request(hr, kp, "hatchery".to_string()).await
}
pub async fn attack(a: Attack, kp: Keypair) -> Result<bool, reqwasm::Error> {
    run_request(a, kp, "hive/attack".to_string()).await
}

async fn run_request<T: Serialize>(t: T, kp: Keypair, url: String) -> Result<bool, reqwasm::Error> {
    let encoded_signature = sign(&t, kp);
    let url = format!("{}/{}", BACKEND, url);
    let bytes = serde_json::to_string(&t).expect("Failed to serialize test data to json");
    let resp = Request::post(&url)
        .body(bytes)
        .header("ed25519-singature", &encoded_signature)
        .header("content-type", "application/json")
        .send()
        .await?;
    Ok(resp.ok())
}

fn sign<T: Serialize>(t: &T, kp: Keypair) -> String {
    let message_string = serde_json::to_string(&t).unwrap();
    let message: &[u8] = message_string.as_bytes();
    let signature: Signature = kp.sign(message);
    bs58::encode(signature).into_string()
}
