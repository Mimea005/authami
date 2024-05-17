use rocket::{catch, catchers, figment::providers::Serialized, fs::FileServer, launch, serde::{Deserialize, Serialize}, Request};
use rocket_dyn_templates::{context, Template};
use templating::TemplateFileServer;
use std::path::PathBuf;

mod templating;

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
#[allow(dead_code)]
struct Config {
    #[allow(dead_code)]
    pub public: PathBuf,
    pub template_dir: PathBuf,
    pub template_page_root: Option<PathBuf>,
    pub use_index_files: bool,
    pub cloak_url: String,
    pub cloak_id: String,
    pub cloak_secret: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            public: PathBuf::from("public"),
            template_dir: PathBuf::from("templates"),
            template_page_root: None,
            use_index_files: false,
            cloak_url: "http://localhost:9011".to_string(),
            cloak_id: "client".to_string(),
            cloak_secret: "secret".to_string(),
        }
    }
}

#[launch]
fn launch() -> _ {
    dotenvy::dotenv().unwrap_or_else(|_| {
        eprintln!("Failed to load .env file");
        return PathBuf::default();
    });
    let default_figment = rocket::figment::Figment::from(Serialized::defaults(Config::default()));
    let figment = default_figment.join(rocket::Config::figment().merge(rocket::figment::providers::Env::prefixed("ROCKET")));
    let rocket_config = figment
        .extract::<rocket::Config>()
        .expect("Invalid Rocket config");
    let custom_config = figment.extract::<Config>().expect("Invalid custom config");

    let file_server = FileServer::from(custom_config.public.clone());
    let template_registry = templating::TemplateRegistry::new(custom_config.template_dir.clone()).unwrap();

    let template_server = TemplateFileServer::builder()
        .template_registry(template_registry)
        .template_page_root(custom_config.template_page_root.clone())
        .use_index_files(custom_config.use_index_files)
        .public_root(custom_config.public.clone())
        .build();

    rocket::custom(figment)
        .attach(Template::fairing())
        .manage(template_registry)
        .manage(rocket_config)
        .manage(custom_config)
        // .mount("/test", routes![test_consume])
        .mount("/", template_server)
        .mount("/", file_server.rank(15)) //Static files
        .register("/", catchers![not_found])
}

#[catch(404)]
fn not_found(req: &Request) -> Template {
    Template::render("404", context! {message: req.uri().path().to_string()})
}
