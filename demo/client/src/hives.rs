#[path = "key_helpers.rs"]
mod key_helpers;

use super::backend::*;
use super::swarm::*;
use sycamore::futures::spawn_local;
use sycamore::prelude::*;
use sycamore::rt::JsCast;
use sycamore::suspense::Suspense;
use web_sys::{Event, KeyboardEvent};

struct SearchByEggs(i64);

#[component]
async fn HivesTables<G: Html>(ctx: ScopeRef<'_>, eggs: i64) -> View<G> {
    let stake_result = ctx.use_context::<RcSignal<StakeResult>>();
    let privatekey = ctx.use_context::<RcSignal<PrivateKey>>();
    let account = ctx.use_context::<RcSignal<Account>>();
    let publickey = ctx.use_context::<RcSignal<SearchPubKey>>();
    let request_failed = ctx.use_context::<RcSignal<bool>>();
    let hives = match super::backend::fetch_hives(eggs).await {
        Ok(h) => h,
        Err(_) => {
            request_failed.set(true);
            return view! { ctx, div{}};
        }
    };
    request_failed.set(false);
    let hives = ctx.create_signal(hives);

    let search_publickey = |s: String| {
        publickey.set(SearchPubKey(s));
    };

    let attack_button = move |s: String| {
        let attack_request = Attack {
            swarm_pubkey: account.get().swarm.pubkey.clone(),
            hive_pubkey: s,
            berserkers: account.get().swarm.berserkers,
        };
        log::info!("Trying to attack with {:?}", attack_request);
        {
            let privatekey = privatekey.clone();
            let stake_result = stake_result.clone();
            spawn_local(async move {
                if let Ok(kp) = key_helpers::get_keypair(privatekey.get().0.to_string()) {
                    stake_result.set(StakeResult(
                        super::backend::attack(attack_request, kp).await,
                    ));
                };
            });
        }
    };

    view! { ctx, div(class="container") { div(class="columns is-multiline is-mobile is-gapless") {
        Indexed {
            iterable: hives,
            view: move |ctx, Hive { pubkey, guardians, queens, eggs }| {
                let p1 = pubkey.clone();
                let p2 = pubkey.clone();
                view! { ctx, div(class="column has-text-centered") {
                    button(
                        class="button is-light is-size-5 is-rounded has-text-success",
                        style="width:520px",
                        on:click=move |_| search_publickey(p1.clone())) {

                        span(class="is-family-code is-size-6 has-text-info",
                            style="text-align:right; width:70px") {
                            "..." (pubkey[pubkey.len() - 4..].to_string())
                        }
                        span(class="icon is-medium has-text-info",
                            style="padding-left:10px") {
                            i(class="fa-lg fa-solid fa-address-card") {}
                        }
                        span(style="text-align:right; width:80px") { (guardians) }
                        span(class="icon is-medium",
                            style="padding-left:5px") {
                            i(class="fa-lg fa-solid fa-shield") {}
                        }
                        span(style="text-align:right; width:60px") { (queens) }
                        span(class="icon is-medium",
                            style="padding-left:5px") {
                            i(class="fa-lg fa-solid fa-chess-queen") {}
                        }
                        span(style="text-align:right; width:70px") { (eggs) }
                        span(class="icon is-medium",
                            style="padding-left:5px") {
                            i(class="fa-lg fa-solid fa-egg") {}
                        }

                        (if account.get().can_attack() {
                            let p2 = p2.clone();
                            view!{ ctx, button(class="button is-danger is-small is-rounded",
                                on:click=move |_| attack_button(p2.clone()),
                                style="margin-left: 15px") { "Attack!" } }
                        } else {
                            view!{ ctx, div {} }
                        })
                    }
                }}
            }
        }
    }}}
}

#[component]
pub async fn HivesComponent<G: Html>(ctx: ScopeRef<'_>) -> View<G> {
    ctx.provide_context(create_rc_signal(false));
    let request_failed = ctx.use_context::<RcSignal<bool>>();
    let eggs = ctx.create_signal(SearchByEggs(0));
    let eggs_input = ctx.create_signal(String::new());
    let reset_eggs = move || {
        eggs_input.set(String::new());
        eggs.set(SearchByEggs(0));
    };
    let set_eggs = move |event: Event| {
        let event: KeyboardEvent = event.unchecked_into();
        if event.key() == "Enter" {
            eggs.set(SearchByEggs(eggs_input.get().parse().unwrap_or_default()));
        }
    };
    let retry_fetch = move || {
        eggs.set(SearchByEggs(eggs.get().0));
    };

    view! { ctx, div {
        div(class="columns is-multiline is-vcentered is-mobile") {
            div(class="column is-narrow") {
                button(class=String::from("button ".to_owned() + (eggs.get().0.eq(&0)
                            .then(|| " is-success")
                            .unwrap_or(" is-light"))),
                            on:click=move |_| reset_eggs()) { "List the best Hives" }
            }

            div(class="column is-auto") {
                div(class="field has-text-success") {
                    p(class="control has-icons-left") {
                        input(class="input is-success",
                            placeholder="Search for hives that have X eggs",
                            bind:value=eggs_input,
                            on:keyup=set_eggs,
                        ) {}
                        span(class="icon is-left has-text-success") {
                            i(class="fa-solid fa-egg"){}
                        }
                    }
                }
            }

            div(class="column is-full") { Suspense {
                fallback: view! { ctx, div(class="notification is-info"){
                    "Loading hive data from blockchain..."
                }},
                children: Children::new(ctx, move |ctx| {
                    view! { ctx,
                    ({
                        let eggs = eggs.get();
                        view! { ctx, HivesTables(eggs.0) }
                    })
                    }
                }),
            }}
            (if *request_failed.get() {
                view! {ctx, div(class="column is-full") {
                    article(class="message is-danger"){
                        div(class="message-header") {
                            p { "Error getting data" }
                            button(class="button is-error is-black",
                                on:click=move |_| retry_fetch()) {
                                "Retry"
                            }
                        }
                        div(class="message-body") {
                            "An error occured when trying to fetch hive data. "
                                "Please sure you have an active internet connection. "
                                "If your internet works, pelase report this bug."
                        }
                    }
                }}
            } else {
                view! { ctx, div {} }
            } )
        }
    }}
}
