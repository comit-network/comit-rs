#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

#[macro_use]
extern crate rocket;
extern crate rocket_contrib;
#[macro_use]
extern crate serde_derive;

use rocket_contrib::Json;

#[derive(Serialize, Deserialize, Debug)]
struct Rate {
    sell: String,
    buy: String,
    rate: f32,
}

#[get("/<sell>/<buy>")]
fn rate(sell: String, buy: String) -> Json<Rate> {
    Json(Rate {
        sell,
        buy,
        rate: 0.5,
    })
}

fn main() {
    rocket::ignite().mount("/rate", routes![rate]).launch();
}
