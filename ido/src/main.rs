use reqwasm::http::Request;
use web3::{
    contract::{Contract, Options},
    transports::eip_1193::{Eip1193, Provider},
    types::{Address, U256},
    Web3,
};

use serde::Serialize;
use sycamore::futures::{spawn_local, spawn_local_scoped};
use sycamore::prelude::*;
use wasm_bindgen::prelude::*;

const BACKEND: &str = "http://localhost:8080/backend";

struct WalletIsConnected(bool);
struct AirdropError(String);

#[derive(Debug, Clone, Copy)]
struct WalletInfo {
    address: Address,
    chain_id: U256,
    balance: f64,
    sacred_queens: u64,
}

#[allow(dead_code)]
enum ContractName {
    SacredQueens,
    Queens,
    Guardians,
    Eggs,
    Berserkers,
}

impl ContractName {
    fn to_string(&self) -> String {
        match self {
            ContractName::SacredQueens => "sacred_queens".to_string(),
            ContractName::Queens => "queens".to_string(),
            _ => "N/A".to_string(),
        }
    }
}

enum FrontendError {
    Backend(String, String),
    Network(reqwasm::Error),
}

async fn get_contract_address(name: ContractName) -> Result<String, FrontendError> {
    let url = format!("{}/contract/{}", BACKEND, name.to_string());
    let resp = Request::get(&url).send().await?;
    let body = resp.text().await?;
    if !resp.ok() {
        return Err(FrontendError::Backend(resp.status_text(), body));
    }
    Ok(body)
}

impl WalletInfo {
    fn new() -> Self {
        WalletInfo {
            address: Address::zero(),
            chain_id: U256::from(0),
            balance: 0.0,
            sacred_queens: 0,
        }
    }
}

fn web3() -> Web3<Eip1193> {
    Web3::new(Eip1193::new(Provider::default().unwrap().unwrap()))
}

async fn contract(web3: &Web3<Eip1193>) -> Result<Contract<Eip1193>, FrontendError> {
    Ok(Contract::from_json(
        web3.eth(),
        get_contract_address(ContractName::SacredQueens)
            .await?
            .parse()
            .unwrap(),
        include_bytes!("../abi/sacred_queens.vy.abi"),
    )
    .unwrap())
}

async fn get_sacred_queens(web3: &Web3<Eip1193>, address: &Address) -> u64 {
    let contract = match contract(&web3).await {
        Ok(c) => c,
        Err(e) => {
            log::info!("Could not get smart contract: {}", e);
            return 0;
        },
    };
    let result = contract
        .query("balanceOf", (*address,), None, Options::default(), None)
        .await;
    let sacred_queens: U256 = match result {
        Ok(u) => u,
        Err(e) => {
            log::info!("Could not get sacred queens: {}", e);
            U256::from(0u64)
        }
    };
    let sacred_queens = sacred_queens
        .checked_div(U256::exp10(18))
        .unwrap_or_else(|| U256::from(0 as u64))
        .as_u64();
    sacred_queens
}

async fn get_wallet_info() -> web3::Result<WalletInfo> {
    let web3 = web3();

    let address = web3.eth().request_accounts().await?[0];
    let chain_id = web3.eth().chain_id().await?;

    let sacred_queens = get_sacred_queens(&web3, &address).await;

    let balance: f64 = web3
        .eth()
        .balance(address, None)
        .await?
        .checked_div(U256::exp10(16))
        .unwrap_or_else(|| U256::from(0 as u64))
        .as_u64() as f64
        / 100.0;

    Ok(WalletInfo {
        address,
        chain_id,
        balance,
        sacred_queens,
    })
}

impl std::fmt::Display for FrontendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            FrontendError::Backend(status, body) => write!(
                f,
                "Backend returned \"{}\". \n{}",
                status, body
            )?,
            FrontendError::Network(e) => write!(f, "Backend request failed: {:?}", e)?,
        };
        Ok(())
    }
}

impl From<reqwasm::Error> for FrontendError {
    fn from(e: reqwasm::Error) -> Self {
        FrontendError::Network(e)
    }
}

async fn get_airdrop() -> Result<String, FrontendError> {
    let web3 = web3();
    let address = web3.eth().request_accounts().await.unwrap()[0];
    let url = format!("{}/airdrop/{:?}", BACKEND, address);
    let resp = Request::get(&url).send().await?;
    let body = resp.text().await?;
    if !resp.ok() {
        return Err(FrontendError::Backend(resp.status_text(), body));
    }
    Ok(body)
}

#[component]
fn ConnectWallet<G: Html>(cx: Scope<'_>) -> View<G> {
    let is_connected = use_context::<RcSignal<WalletIsConnected>>(cx);
    let wallet_info = use_context::<RcSignal<WalletInfo>>(cx);

    let get_account_button = move |_| {
        spawn_local_scoped(cx, async move {
            match get_wallet_info().await {
                Ok(wi) => {
                    wallet_info.set(wi);
                    is_connected.set(WalletIsConnected(true));
                }
                Err(e) => log::info!("Could not get wallet info: {}", e),
            }
        });
    };

    view!(cx, div {
        div(class="has-text-info has-text-centered is-size-5") {
            br {}
            p { "MetaMask wallet detected!" }
            br {}
            div(
                class="button is-medium is-outlines is-black has-text-info",
                on:click=get_account_button) {
                "Connect!"
            }
        }
    })
}

#[component]
fn WalletInfo<G: Html>(cx: Scope<'_>) -> View<G> {
    let wallet_info = use_context::<RcSignal<WalletInfo>>(cx);
    provide_context(cx, create_rc_signal(AirdropError(String::new())));
    let airdrop_error = use_context::<RcSignal<AirdropError>>(cx);

    let get_airdrop_button = move |_| {
        let wallet_info = wallet_info.clone();
        let airdrop_error = airdrop_error.clone();
        spawn_local(async move {
            match get_airdrop().await {
                Ok(r) => {
                    log::info!("Got airdrop: {:?}", r);
                    let mut wi: WalletInfo = *wallet_info.get();
                    wi.sacred_queens += 100;
                    wallet_info.set(wi);
                }
                Err(e) => {
                    log::info!("Could not get airdrop: {}", e);
                    airdrop_error.set(AirdropError(e.to_string()));
                }
            }
        });
    };

    view!(cx, div {
        p { "Connected!" }
        br {}
        (if !wallet_info.get().chain_id.eq(&U256::from(1337 as u64)) {
            view!(cx, div(class="notification") {
                "You are on the wrong chain! Switch to localhost!"
            })
        } else {
            view!(cx, div() {})
        })
        table(class="container table is-info") {
            tr {
                th {"Chain:" }
                th { (wallet_info.get().chain_id) }
            }
            tr {
                th {"Address:" }
                th { (wallet_info.get().address.to_string()) }
            }
            tr {
                th {"Balance:" }
                th { (wallet_info.get().balance) }
            }
            tr {
                th {"Sacred Queens:" }
                th { (wallet_info.get().sacred_queens) }
            }
        }
        (if wallet_info.get().sacred_queens < 100 {
            view!(cx, div(
                class="button is-medium is-outlines is-black has-text-info",
                on:click=get_airdrop_button) {
                "Get airdrop!"
            })
        } else {
            view!(cx, div(class="has-text-success") {
                "You already have enough Sacred Queens to play the game!"
            })
        })
        (if airdrop_error.get().0.len() > 0 {
            view!(cx, div(class="has-text-danger") {
                "Failed to get airdrop: " (airdrop_error.get().0)
            })
        } else {
            view!(cx, div() {})
        })
    })
}

#[component]
fn WalletIntegration<G: Html>(cx: Scope<'_>) -> View<G> {
    provide_context(cx, create_rc_signal(WalletIsConnected(false)));
    provide_context(cx, create_rc_signal(WalletInfo::new()));
    let is_connected = use_context::<RcSignal<WalletIsConnected>>(cx);

    view! { cx,
        div(class="has-text-centered is-size-5") {
            (if is_connected.get().0 {
                view!(cx, WalletInfo{})
            } else {
                view!(cx, ConnectWallet{})
            } )
        }
    }
}

#[component]
fn WalletNotFound<G: Html>(cx: Scope<'_>) -> View<G> {
    view!(cx, div {
        div(class="has-text-danger has-text-centered is-size-5") {
            br {}
            p { "MetaMask wallet not detected!" }
            br {}
            a(href="https://metamask.io/", target="_blank") {
                div(class="button is-medium is-outlines is-black has-text-danger") { "Get MetaMask!" }
            }
        }
    })
}

#[component]
fn App<G: Html>(cx: Scope<'_>) -> View<G> {
    let metamask_detected = create_signal(cx, ethereum_is_defined());

    view! { cx, div(class="container has-text-centered", style="font-family: 'Recursive', monospace;") {
        br {}
        div(class="title"){ "Sacred Queens Airdrop" }
        (if *metamask_detected.get() {
            view!(cx, WalletIntegration{})
        } else {
            view!(cx, WalletNotFound{})
        } )
        br {}
    }}
}

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();
    sycamore::render(|cx| view! { cx, App() });
}

#[derive(Serialize)]
pub struct MetaMaskRequest {
    pub method: String,
}

#[wasm_bindgen]
extern "C" {
    pub type Ethereum;
    #[wasm_bindgen(js_name = ethereum)]
    pub static ETHEREUM: Ethereum;
    #[wasm_bindgen(method, catch, getter=isMetaMask)]
    pub fn is_metamask(this: &Ethereum) -> Result<bool, JsValue>;
    #[wasm_bindgen(method)]
    pub async fn request(this: &Ethereum, request: &JsValue) -> JsValue;
}

#[wasm_bindgen(module = "/js/ethereum.js")]
extern "C" {
    #[wasm_bindgen]
    fn ethereum_is_defined() -> bool;
}
