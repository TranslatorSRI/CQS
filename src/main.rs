#[macro_use]
extern crate rocket;

pub mod model;

use crate::model::QueryGraph;
use rocket::serde::{json::Json, Deserialize};
use serde_json::Value;
use std::collections::HashMap;

#[post("/query", data = "<query>")]
fn hello(query: String) -> String {
    format!("Hello!")
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/hello", routes![hello])
}
