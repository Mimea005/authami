use std::{collections::HashMap, path::PathBuf};
use rocket::{form::Form, fs::FileServer, get, http::RawStr, launch, routes, serde::{self, Deserialize}, FromForm, State};

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
#[allow(dead_code)]
struct Config {
    #[allow(dead_code)]
    pub public: PathBuf,
    pub cloak_url: String,
    pub cloak_id: String,
    pub cloak_secret: String,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            public: PathBuf::from("public"),
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
    let figment = rocket::Config::figment();
    let rocket_config = figment.extract::<rocket::Config>().expect("Invalid Rocket config");
    let custom_config = figment.extract::<Config>().expect("Invalid custom config");
    let file_server = FileServer::from(custom_config.public.clone());
    rocket:: custom(figment)
        .mount("/", file_server)
        .mount("/api", routes![index, hello, callback])
        .mount("/hx", routes![hx::signin])
        .manage(rocket_config)
        .manage(custom_config)
}

#[get("/")]
fn index() -> &'static str {
    "Who, are you!"
}

#[get("/hello")]
fn hello() -> &'static str {
    "I don't know you"
}

#[derive(FromForm, Debug)]
#[allow(dead_code)]
struct CallbackParams<'r> {
    code: &'r str,
    locale: &'r str,
    #[field(name = "userState")]
    user_state: &'r str,
}

#[get("/callback?<params..>")]
async fn callback(params: CallbackParams<'_>, config: &State<Config>) -> String {
    let mut form =HashMap::new();
    form.insert("code", params.code);
    form.insert("client_id", &config.cloak_id);
    form.insert("client_secret", &config.cloak_secret);
    form.insert("grant_type", "authorization_code");
    form.insert("redirect_uri", "http://localhost:8000/api/callback");

    let client = reqwest::Client::new();

    match client.post(format!("{}/oauth2/token", config.cloak_url))
        .form(&form)
        .send()
        .await {
        Ok(res) => return format!("{:#?}", res),
        Err(err) => return format!("{:#?}", err),
    }
}

mod hx {
    use super::Config;
    use rocket::{http::RawStr, get, State};

    #[get("/signin")]
    pub fn signin(config: &State<Config>, rocket_config: &State<rocket::Config>) -> String {
        let rawStr = format!("http://{}:{}/api/callback", "localhost", rocket_config.port);
        format! ( 
        "<a href=\"http://{}/oauth2/authorize?client_id={}&response_type=code&redirect_uri={}\">Sign in<a>" ,
            // config.cloak_url,
            "localhost:9011",
            config.cloak_id,
            RawStr::new(&rawStr).percent_encode()
        )
    }
}

