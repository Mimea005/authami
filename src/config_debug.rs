use std::collections::HashMap;

use log::{error, info};
use rocket::{
    fairing::{Fairing, Result},
    Build, Rocket,
};
use yansi::Paint;

pub struct ConfigDebug;

#[rocket::async_trait]
impl Fairing for ConfigDebug {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "Config Debug",
            kind: rocket::fairing::Kind::Ignite,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> Result {
        info!("{} {}", "🛠".mask(), "Config Debug".bold().magenta());
        match rocket.figment().extract::<serde_json::Value>() {
            Err(e) => {
                error!(target:"_", "{} {:#?}", "Failed to extract config:", e);
                Err(rocket)
            }
            Ok(all_config) => {
                let profile = rocket.figment().profile();
                info!("{} {}", "Profile:".yellow().bold(), profile);
                info!("\n{all_config:#?}");
                Ok(rocket)
            }
        }
    }
}
