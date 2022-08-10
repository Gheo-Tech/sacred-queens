use actix_web::{
    error, get,
    http::{header::ContentType, StatusCode},
    web, App, HttpResponse, HttpServer, Result,
};
use once_cell::sync::Lazy;
use secp256k1::SecretKey;
use std::{str::FromStr, sync::Arc};
use web3::{
    contract::{Contract, Options},
    transports::WebSocket,
    types::{Address, U256},
    Web3,
};

const BERSERKERS: &str = "berserkers";
const EGGS: &str = "eggs";
const GUARDIANS: &str = "guardians";
const QUEENS: &str = "queens";
const SACRED_QUEENS: &str = "sacred_queens";

static SQ_ADDR: Lazy<String> = Lazy::new(|| std::env::var("SQ_ADDR").unwrap());
static Q_ADDR: Lazy<String> = Lazy::new(|| std::env::var("Q_ADDR").unwrap());

enum ContractName {
    SacredQueens,
    Queens,
    Guardians,
    Eggs,
    Berserkers,
}

#[derive(Debug)]
enum BackendError {
    ContractNotFound,
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            BackendError::ContractNotFound => write!(f, "Contract name not found"),
        }
    }
}

impl error::ResponseError for BackendError {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            BackendError::ContractNotFound => StatusCode::NOT_FOUND,
        }
    }
}

impl TryFrom<String> for ContractName {
    type Error = BackendError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            BERSERKERS => Ok(ContractName::Berserkers),
            EGGS => Ok(ContractName::Eggs),
            GUARDIANS => Ok(ContractName::Guardians),
            QUEENS => Ok(ContractName::Queens),
            SACRED_QUEENS => Ok(ContractName::SacredQueens),
            _ => Err(BackendError::ContractNotFound),
        }
    }
}

impl ContractName {
    fn address(&self) -> &str {
        match self {
            ContractName::SacredQueens => &SQ_ADDR,
            ContractName::Queens => &Q_ADDR,
            _ => "",
        }
    }
}

#[derive(Clone)]
struct Web3Config {
    web3: Web3<WebSocket>,
    contract: Contract<WebSocket>,
    key: Arc<SecretKey>,
}

#[get("/contract/{name}")]
async fn get_contract_address(name: web::Path<String>) -> Result<HttpResponse> {
    let contract: ContractName = ContractName::try_from(name.into_inner())?;
    Ok(HttpResponse::Ok().body(format!("{}", contract.address())))
}

#[get("/airdrop/{pubkey}")]
async fn get_airdrop(web3config: web::Data<Web3Config>, pubkey: web::Path<String>) -> HttpResponse {
    let pubkey = pubkey.into_inner();
    let address = Address::from_str(&pubkey).unwrap();

    let balance = web3config
        .contract
        .query("balanceOf", (address,), None, Options::default(), None)
        .await;

    let sacred_queens: U256 = match balance {
        Ok(u) => u,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .body(format!("failed to get wallet balance: {:?}", e))
        }
    };

    let airdrop_value = U256::from(100).checked_mul(U256::exp10(18)).unwrap();
    if sacred_queens >= airdrop_value {
        return HttpResponse::Unauthorized().body(format!(
            "Wallet {} already has {} Sacred Queens.",
            pubkey, sacred_queens
        ));
    };

    match web3config
        .contract
        .signed_call_with_confirmations(
            "transfer",
            (address, airdrop_value),
            Options::default(),
            0,
            web3config.key.clone(),
        )
        .await
    {
        Err(e) => HttpResponse::BadGateway().body(format!("{e}")),
        Ok(a) => HttpResponse::Ok().body(format!("{a:?}")),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Sacred Queens contract address: {}", *SQ_ADDR);
    println!("Queens contract address: {}", *Q_ADDR);
    let transport = web3::transports::WebSocket::new("ws://192.168.1.182:8545")
        .await
        .unwrap();
    let web3 = Web3::new(transport);
    let contract = Contract::from_json(
        web3.eth(),
        SQ_ADDR.parse().unwrap(),
        include_bytes!("../../contracts/vyper/target/sacred_queens.vy.abi"),
    )
    .unwrap();
    let key = Arc::new(
        SecretKey::from_str(
            &std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY env var is mandatory"),
        )
        .unwrap(),
    );
    let web3config = Web3Config {
        web3,
        contract,
        key,
    };

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(web3config.clone()))
            .service(get_airdrop)
            .service(get_contract_address)
    })
    .bind(("127.0.0.1", 9000))?
    .run()
    .await
}
