mod hives;
mod swarm;
mod backend;

use sycamore::prelude::*;
use crate::backend::Account;

#[component]
fn App<G: Html>(ctx: ScopeRef) -> View<G> {
    ctx.provide_context(create_rc_signal(Account::new()));
    ctx.provide_context(create_rc_signal(swarm::SearchPubKey(String::new())));
    view! { ctx, div(class="columns is-mobile is-multiline section") {
        div(class="column") {
            div(class="container", style="width:520px;") {
                swarm::SwarmComponent{}
                br(){}
            }
        }
        div(class="column") {
            div(class="container", style="width:520px;") {
                hives::HivesComponent{}
            }
        }
    }}
}

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    sycamore::render(|ctx| view! { ctx, App {} });
}
