use authami::{template_routes, TemplateServer};
use rocket::{
    catch, catchers,
    figment::{providers::Serialized, Figment},
    fs::FileServer,
    launch, routes,
    serde::{Deserialize, Serialize},
    Request,
};
use rocket_dyn_templates::{context, Template};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
#[allow(dead_code)]
struct Config {
    pub public: PathBuf,
    // pub cloak_url: String,
    // pub cloak_id: String,
    // pub cloak_secret: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            public: PathBuf::from("public"),
            // cloak_url: "http://localhost:9011".to_string(),
            // cloak_id: "client".to_string(),
            // cloak_secret: "secret".to_string(),
        }
    }
}

#[launch]
fn launch() -> _ {
    dotenvy::dotenv().unwrap_or_else(|_| {
        eprintln!("Failed to load .env file");
        return PathBuf::default();
    });

    let figment =
        Figment::from(Serialized::defaults(Config::default())).merge(rocket::Config::figment());

    let file_server = FileServer::from(Config::default().public);

    rocket::custom(figment)
        .attach(Template::fairing())
        .attach(TemplateServer)
        .mount("/", routes![template_routes])
        .mount("/", file_server.rank(15)) //Static files
        .register("/", catchers![not_found])
}

#[catch(404)]
fn not_found(req: &Request) -> Template {
    Template::render("404", context! {message: req.uri().path().to_string()})
}
