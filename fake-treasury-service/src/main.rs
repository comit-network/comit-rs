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
    symbol: String,
    rate: f32,
}

#[get("/<symbol>")]
fn rate(symbol: String) -> Json<Rate> {
    Json(Rate {
        symbol,
        rate: 0.5
    })
}

fn main() {
    rocket::ignite().mount("/rate", routes![rate]).launch();
}
