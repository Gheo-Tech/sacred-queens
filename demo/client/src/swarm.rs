#[path = "key_helpers.rs"]
mod key_helpers;

use gloo_timers::future::TimeoutFuture;

use super::backend::*;
use sycamore::futures::spawn_local;
use sycamore::prelude::*;
use sycamore::rt::JsCast;
use sycamore::suspense::Suspense;
use web_sys::{Event, KeyboardEvent};

pub struct SearchPubKey(pub String);
pub struct PrivateKey(pub String);

pub struct AssetsConfig {
    pubkey: String,
    owned: bool,
}

#[component]
pub async fn Assets<G: Html>(ctx: ScopeRef<'_>, c: AssetsConfig) -> View<G> {
    if c.owned {
        TimeoutFuture::new(50).await;
    }
    if c.pubkey == "" {
        return view! { ctx, div{} };
    }
    let account = match get_account(c.pubkey).await {
        Ok(a) => a,
        _ => {
            return view! { ctx, div{
                    article(class="message is-danger"){
                        div(class="message-header") {
                            p { "Error getting data" }
                        }
                        div(class="message-body") {
                            "An error occured when trying to fetch swarm data. "
                            "Please sure you have an active internet connection. "
                            "If your internet works, pelase report this bug."
                        }
                    }
            } };
        }
    };
    let account_ctx = ctx.use_context::<RcSignal<Account>>();
    if c.owned {
        account_ctx.set(account.clone());
    }

    macro_rules! show_asset {
        ($class:expr, $asset:expr, $icon:expr) => {{
            let mut span_class: String = String::from("tag is-size-5 is-rounded has-text-");
            span_class.push_str($class);
            let mut i_class: String = String::from("fa fa-solid fa-");
            i_class.push_str($icon);
            if ($asset == 0) {
                view! { ctx, div{} }
            } else {
                view! { ctx, span(class=span_class) {
                    span{
                        ($asset)
                    }
                    span(class="icon is-medium") {
                        i(class=i_class) {}
                    }
                }}
            }
        }};
    }

    view! { ctx, div(class="column is-full") {
        div(class="tags are-small") {
            (show_asset!("dark", account.swarm.eggs, "egg"))
            (show_asset!("dark", account.swarm.sacred_queens, "chess-king"))
            (show_asset!("dark", account.swarm.queens, "chess-queen"))
            (show_asset!("dark", account.swarm.guardians, "shield"))
            (show_asset!("dark", account.swarm.berserkers, "shield-virus"))
            (show_asset!("danger", account.sacred_hive.eggs, "egg"))
            (show_asset!("danger", account.sacred_hive.sacred_queens, "chess-king"))
            (show_asset!("success", account.hive.eggs, "egg"))
            (show_asset!("success", account.hive.queens, "chess-queen"))
            (show_asset!("success", account.hive.guardians, "shield"))
        }
    }}
}

#[component]
pub async fn Staking<G: Html>(ctx: ScopeRef<'_>) -> View<G> {
    let account = ctx.use_context::<RcSignal<Account>>();
    let privatekey = ctx.use_context::<RcSignal<PrivateKey>>();

    let stake_result = ctx.use_context::<RcSignal<StakeResult>>();
    let hatch_eggs = ctx.create_signal(String::new());
    let unstake_s_queens = ctx.create_signal(String::new());
    let unstake_s_eggs = ctx.create_signal(String::new());
    let stake_s_queens = ctx.create_signal(String::new());
    let stake_s_eggs = ctx.create_signal(String::new());
    let unstake_queens = ctx.create_signal(String::new());
    let unstake_eggs = ctx.create_signal(String::new());
    let unstake_guardians = ctx.create_signal(String::new());
    let stake_queens = ctx.create_signal(String::new());
    let stake_eggs = ctx.create_signal(String::new());
    let stake_guardians = ctx.create_signal(String::new());

    let hatch_eggs_button = move || {
        let hatch_request = HatchRequest {
            pubkey: account.get().swarm.pubkey.clone(),
            eggs: hatch_eggs.get().parse().unwrap_or(0),
        };
        hatch_eggs.set(String::new());
        {
            let privatekey = privatekey.clone();
            let stake_result = stake_result.clone();
            spawn_local(async move {
                if let Ok(kp) = key_helpers::get_keypair(privatekey.get().0.to_string()) {
                    stake_result.set(StakeResult(super::backend::hatch(hatch_request, kp).await));
                };
            });
        }
    };

    macro_rules! s_stake_operation {
        ($sacred_queens_input:expr, $eggs_input:expr, $operation:ident) => {{
            let sacred_hive = SacredHive {
                pubkey: account.get().swarm.pubkey.clone(),
                sacred_queens: $sacred_queens_input.get().parse().unwrap_or(0),
                eggs: $eggs_input.get().parse().unwrap_or(0),
            };
            $sacred_queens_input.set(String::new());
            $eggs_input.set(String::new());
            {
                let privatekey = privatekey.clone();
                let stake_result = stake_result.clone();
                spawn_local(async move {
                    if let Ok(kp) = key_helpers::get_keypair(privatekey.get().0.to_string()) {
                        stake_result.set(StakeResult(
                            super::backend::$operation(sacred_hive, kp).await,
                        ));
                    }
                });
            }
        }};
    }

    let unstake_sacred_hive_button = move || {
        s_stake_operation!(unstake_s_queens, unstake_s_eggs, unstake_sacred_hive);
    };
    let stake_sacred_hive_button = move || {
        s_stake_operation!(stake_s_queens, stake_s_eggs, stake_sacred_hive);
    };

    macro_rules! stake_operation {
        ($queens_input:expr, $guardians_input:expr, $eggs_input:expr, $operation:ident) => {{
            let hive = Hive {
                pubkey: account.get().swarm.pubkey.clone(),
                queens: $queens_input.get().parse().unwrap_or(0),
                guardians: $guardians_input.get().parse().unwrap_or(0),
                eggs: $eggs_input.get().parse().unwrap_or(0),
            };
            $queens_input.set(String::new());
            $eggs_input.set(String::new());
            {
                let privatekey = privatekey.clone();
                let stake_result = stake_result.clone();
                spawn_local(async move {
                    if let Ok(kp) = key_helpers::get_keypair(privatekey.get().0.to_string()) {
                        stake_result.set(StakeResult(super::backend::$operation(hive, kp).await));
                    };
                });
            }
        }};
    }

    let unstake_hive_button = move || {
        stake_operation!(
            unstake_queens,
            unstake_guardians,
            unstake_eggs,
            unstake_hive
        );
    };
    let stake_hive_button = move || {
        stake_operation!(stake_queens, stake_guardians, stake_eggs, stake_hive);
    };

    macro_rules! show_field {
        ($bind_value:expr, $placeholder:expr, $icon:expr, $input_color:expr) => {{
            let _some_var = String::new();
            view! { ctx, div(class="column") {
                div(class=("field has-text-".to_string() + $input_color)) {
                    div(class="control has-icons-left") {
                        input(
                            class=("input is-".to_string() + $input_color),
                            bind:value=$bind_value,
                            max="100", min="0",
                            type="number", placeholder=$placeholder) {}
                        span(class=("icon is-left has-text-".to_string() +
                                $input_color)) {
                            i(class=$icon){}
                        }
                    }
                }
            }}
        }};
    }

    view! { ctx, div(class="column is-full") {
        (match (*stake_result.get()).0 {
            Ok(true) => view! { ctx, div {}},
            _ => view! { ctx, div(class="column is-full") {
                div(class="notificaiton is-danger") {
                    article(class="message is-danger"){
                        div(class="message-body") {
                            i(class="fa-solid fa-skull") {}
                            " Stake request failed! Please make sure you have enough tokens and try again!"
                        }
                    }
                }
            }},
        })

        div(class="columns is-mobile is-variable is-1",
            style=String::from("margin-bottom: -20px; margin-top: -20px; ".to_owned()
                + (account.get().can_hatch().then(|| "").unwrap_or("display: none")))) {
            (show_field!(hatch_eggs, "Hatch Eggs", "fa-solid fa-egg", "dark"))
            div(class="column is-2") {
                button(class="button is-light is-rounded is-fullwidth", style="text-weight: bold",
                    on:click=move |_| hatch_eggs_button()) { "hatch" }
            }
        }

        div(class="columns is-mobile is-variable is-1",
            style=String::from("margin-bottom: -20px; margin-top: -20px; ".to_owned()
                + (account.get().can_unstake_s().then(|| "").unwrap_or("display: none")))) {
            (show_field!(unstake_s_queens, "Sacred Queens", "fa-solid fa-chess-king", "danger"))
            (show_field!(unstake_s_eggs, "Eggs", "fa-solid fa-egg", "danger"))
            div(class="column is-2") {
                button(class="button is-light is-rounded is-fullwidth has-text-danger",
                    on:click=move |_| unstake_sacred_hive_button()) { "collect" }
            }
        }

        div(class="columns is-mobile is-variable is-1",
            style=String::from("margin-bottom: -20px; margin-top: -20px; ".to_owned()
                + (account.get().can_stake_s().then(|| "").unwrap_or("display: none")))) {
            (show_field!(stake_s_queens, "Sacred Queens", "fa-solid fa-chess-king", "dark"))
            (show_field!(stake_s_eggs, "Eggs", "fa-solid fa-egg", "dark"))
            div(class="column is-2") {
                button(class="button is-light is-rounded is-fullwidth",
                    on:click=move |_| stake_sacred_hive_button()) { "brood" }
            }
        }

        div(class="columns is-mobile is-variable is-1",
            style=String::from("margin-bottom: -20px; margin-top: -20px; ".to_owned()
                + (account.get().can_unstake_h().then(|| "").unwrap_or("display: none")))) {
            (show_field!(unstake_queens, "Queens", "fa-solid fa-chess-queen", "success"))
            (show_field!(unstake_eggs, "Eggs", "fa-solid fa-egg", "success"))
            (show_field!(unstake_guardians, "Guardians", "fa-solid fa-shield", "success"))
            div(class="column is-2") {
                button(class="button is-light is-rounded is-fullwidth has-text-success",
                    on:click=move |_| unstake_hive_button()) { "collect" }
            }
        }

        div(class="columns is-mobile is-variable is-1",
            style=String::from("margin-bottom: -20px; margin-top: -20px; ".to_owned()
                + (account.get().can_stake_h().then(|| "").unwrap_or("display: none")))) {
            (show_field!(stake_queens, "Queens", "fa-solid fa-chess-queen", "dark"))
            (show_field!(stake_eggs, "Eggs", "fa-solid fa-egg", "dark"))
            (show_field!(stake_guardians, "Guardians", "fa-solid fa-shield", "dark"))
            div(class="column is-2") {
                button(class="button is-light is-rounded is-fullwidth",
                    on:click=move |_| stake_hive_button()) { "brood" }
            }
        }

    }}
}

#[component]
pub async fn Wallet<'a, G: Html>(ctx: ScopeRef<'a>) -> View<G> {
    ctx.provide_context(create_rc_signal(PrivateKey(String::new())));
    let privatekey = ctx.use_context::<RcSignal<PrivateKey>>();
    ctx.provide_context(create_rc_signal(AirdropResult(Ok(true))));
    let airdrop_result = ctx.use_context::<RcSignal<AirdropResult>>();
    let privatekey_input = ctx.create_signal(String::new());
    let publickey = ctx.create_signal(String::new());
    ctx.provide_context(create_rc_signal(StakeResult(Ok(true))));
    let stake_result = ctx.use_context::<RcSignal<StakeResult>>();

    let privatekey_event = move |event: Event| {
        let event: KeyboardEvent = event.unchecked_into();
        if privatekey_input.get().is_empty() {
            publickey.set(String::new())
        } else if event.key() == "Enter" {
            if let Ok(kp) = key_helpers::get_keypair(privatekey_input.get().to_string()) {
                privatekey.set(PrivateKey(privatekey_input.get().to_string()));
                publickey.set(key_helpers::get_pubkey(&kp))
            };
        }
    };

    ctx.create_effect(|| match airdrop_result.get().0 {
        Ok(true) => {}
        _ => {
            privatekey_input.set(String::new());
            publickey.set(String::new());
        }
    });

    ctx.create_effect(|| match stake_result.get().0 {
        Ok(true) => {
            let new_string = publickey.get().to_string();
            spawn_local(async move {
                super::backend::get_trigger(new_string.to_string()).await;
            });
            publickey.set(publickey.get().to_string());
        }
        _ => {}
    });

    let get_airdrop_button = move || {
        let keypair = key_helpers::generate_keypair();
        let privkey = key_helpers::get_privkey(&keypair);
        let pubkey = key_helpers::get_pubkey(&keypair);
        privatekey_input.set(privkey.clone());
        let p = pubkey.clone();
        {
            let airdrop_result = airdrop_result.clone();
            spawn_local(async move {
                airdrop_result.set(AirdropResult(get_airdrop(p).await));
            });
        }
        publickey.set(pubkey.clone());
        privatekey.set(PrivateKey(privatekey_input.get().to_string()));
    };

    view! { ctx, div(class="columns is-mobile is-multiline") {
        div(class="column is-full") {
            div(class="subtitle is-4") {
                "Inspect your own swarm..."
            }
        }
        (match (*airdrop_result.get()).0 {
            Ok(true) => view! { ctx, div {}},
            _ => view! { ctx, div(class="column is-full") {
                div(class="notificaiton is-danger") {
                    article(class="message is-danger"){
                        div(class="message-body") {
                            i(class="fa-solid fa-skull") {}
                            " Failed to get airdrop from blockchain! Please try again later!"
                        }
                    }
                }
            }},
        })
        (if privatekey_input.get().is_empty() {
            view! { ctx, div(class="column is-narrow") {
                button(class="button is-link",
                    on:click=move |_| get_airdrop_button()) {
                    "Get airdrop!"
                }
            }}
        } else {
            view! { ctx, div {}}
        })
        div(class="column") {
            div(class="field has-text-danger") {
                p(class="control has-icons-left") {
                    input(class="input is-danger is-family-code",
                        placeholder="... or paste your private key!",
                        bind:value=privatekey_input,
                        on:keyup=privatekey_event) {}
                    span(class="icon is-left has-text-danger") {
                        i(class="fas fa-key"){}
                    }
                }
            }
        }
        div(class="column is-full has-text-right") {
            span(class="icon has-text-warning") {
                i(class="fa-solid fa-triangle-exclamation") {}
            }
            span {
                " Remember to keep your private key safe! "
            }
        }
        div(class="column is-full") {
            div(class="field") {
                p(class="control has-icons-left") {
                    input(class="input is-light is-family-code has-text-grey-light",
                        readonly=true,
                        value=publickey.get(),
                        placeholder="Your public key will appear here.") {}
                    span(class="icon is-left has-text-grey-light") {
                        i(class="fa-solid fa-address-card"){}
                    }
                }
            }
        }
        Suspense {
            children: Children::new(ctx, move |ctx| {
                view! { ctx, ({
                    let p = publickey.get().to_string();
                    view! { ctx, Assets(AssetsConfig { pubkey: p, owned: true }) }
                })}
            }),
        }
        Staking {}
        div(class="column is-full") {}
    }}
}

#[component]
pub async fn Search<'a, G: Html>(ctx: ScopeRef<'a>) -> View<G> {
    let publickey = ctx.use_context::<RcSignal<SearchPubKey>>();
    let publickey_input = ctx.create_signal(publickey.get().0.clone());

    let swarm_data = ctx.provide_context(create_rc_signal(Swarm::new()));

    let publickey_event = move |event: Event| {
        let event: KeyboardEvent = event.unchecked_into();
        if event.key() == "Enter" {
            publickey.set(SearchPubKey(publickey_input.get().to_string()));
        }
    };

    ctx.create_effect(move || {
        let pubkey = publickey.get().0.clone();
        publickey_input.set(pubkey.clone());
        {
            let swarm_data = swarm_data.clone();
            let pubkey = pubkey.clone();
            spawn_local(async move {
                if let Ok(s) = get_swarm(pubkey).await {
                    swarm_data.set(s);
                };
            });
        }
    });

    view! { ctx, div(class="columns is-mobile is-multiline") {
        div(class="column is-full") {
            div(class="subtitle is-4") {
                "Or inspect another swarm..."
            }
        }
        div(class="column is-full") {
            div(class="field has-text-info") {
                p(class="control has-icons-left") {
                    input(class="input is-info is-family-code",
                        bind:value=publickey_input,
                        placeholder="Search by public key...",
                        on:keyup=publickey_event) {}
                    span(class="icon is-left has-text-info") {
                        i(class="fa-solid fa-address-card"){}
                    }
                }
            }
        }
        Suspense {
            children: Children::new(ctx, move |ctx| {
                view! { ctx, ({
                    let p = publickey.get().0.to_string();
                    view! { ctx, Assets(AssetsConfig { pubkey: p, owned: false }) }
                })}
            }),
        }
    }}
}

#[component]
pub async fn SwarmComponent<'a, G: Html>(ctx: ScopeRef<'a>) -> View<G> {
    view! { ctx, div(class="columns is-multiline") {
        div(class="column is-full") { Wallet{} }
        div(class="column is-full") { Search{} }
    }}
}
