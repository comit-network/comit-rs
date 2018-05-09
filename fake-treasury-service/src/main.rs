#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate rocket;

#[get("/<sell>/<buy>")]
fn rate(sell: String, buy: String) -> String {
    format!("Hello, {} year old named {}!", sell, buy)
}

fn main() {
    rocket::ignite().mount("/rate", routes![rate]).launch();
}
