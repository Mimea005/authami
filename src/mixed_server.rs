use std::{
    collections::{HashMap, VecDeque},
    ffi::OsStr,
    path::PathBuf,
};

use log::{info, trace, warn};
use rocket::{
    fairing::{self, Fairing, Info},
    figment::{providers::Serialized, Figment},
    get,
    http::Status,
    request::{FromRequest, Outcome},
    serde::{Deserialize, Serialize},
    Rocket,
};
use rocket_dyn_templates::{context, Template};
use yansi::Paint;

pub struct TemplateServer;

#[rocket::async_trait]
impl Fairing for TemplateServer {
    fn info(&self) -> Info {
        Info {
            name: "Mixed Server",
            kind: fairing::Kind::Ignite | fairing::Kind::Request | fairing::Kind::Liftoff,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<rocket::Build>) -> fairing::Result {
        let mut rocket = rocket;
        let figment = Figment::from(Serialized::defaults(TemplateServerConfig::default()))
            .merge(rocket.figment());
        rocket = rocket.configure(figment);

        // let config = rocket.figment().extract::<MixedServerConfig>().expect("Missing config fo MixedServer");
        let root_directory = rocket
            .figment()
            .extract_inner::<PathBuf>("template_dir")
            .expect("Missing config fo Templating");

        let mut registry = TemplateRegistry::new();

        let mut queue = VecDeque::from(vec![root_directory.clone()]);
        while let Some(entry) = queue.pop_front() {
            if entry.is_dir() {
                if let Ok(subdirs) = entry.read_dir().map(|entries| {
                    entries
                        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                        .collect::<Vec<_>>()
                }) {
                    queue.extend(subdirs)
                }
            } else if entry.is_file() {
                if entry.extension() == Some(OsStr::new("hbs").as_ref()) {
                    let template_name = remove_extension(
                        entry.strip_prefix(&root_directory).unwrap().to_path_buf(),
                    )
                    .to_str()
                    .unwrap()
                    .to_string();
                    registry.inner_mut().insert(
                        template_name.clone(),
                        Box::leak(Box::new(template_name.clone())),
                    );
                    info!(target: "_", "Register template: {}", template_name);
                }
            }
        }
        rocket = rocket.manage(registry);

        Ok(rocket)
    }

    async fn on_liftoff(&self, rocket: &Rocket<rocket::Orbit>) {
        let config = rocket
            .figment()
            .extract::<TemplateServerConfig>()
            .expect("Missing config fo MixedServer");
        let root_directory = rocket
            .figment()
            .extract_inner::<PathBuf>("template_dir")
            .expect("Missing config fo Templating");
        let registry = rocket
            .state::<TemplateRegistry>()
            .expect("Missing TemplateRegistry");
        info!("{} {}", "👷".mask(), "Mixed Server".magenta());
        info!(target: "_", "Use index files: {}", config.use_index_files);
        info!(target: "_", "Directory: {root_directory:?}");
        info!(target: "_", "Templates: {}", registry.inner().len());
    }

    async fn on_request(&self, req: &mut rocket::Request<'_>, _data: &mut rocket::Data<'_>) {
        trace!(target:"_", "Request: {:?}", req);
    }
}

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct TemplateServerConfig {
    use_index_files: bool,
    // custom_template_page_sub_root: Option<PathBuf>, //TODO: Add custom page folder handling
    // public_root: PathBuf, //TODO: Add public files handling
}

impl Default for TemplateServerConfig {
    fn default() -> Self {
        Self {
            use_index_files: false,
            // custom_template_page_sub_root: None,
            // public_root: PathBuf::default(),
        }
    }
}

struct TemplateRegistry(HashMap<String, &'static str>);

impl TemplateRegistry {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn inner(&self) -> &HashMap<String, &'static str> {
        &self.0
    }

    pub fn inner_mut(&mut self) -> &mut HashMap<String, &'static str> {
        &mut self.0
    }
}

pub struct Templated(pub &'static str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Templated {
    type Error = String;
    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let segments: PathBuf = request.segments(0..).unwrap();
        trace!(target:"_", "Segments: {:?}", segments);

        let registry = request
            .rocket()
            .state::<TemplateRegistry>()
            .expect("Missing TemplateRegistry");

        if let Some(path) = segments.to_str() {
            let template_name = path.to_string();
            trace!(target:"_", "Requested template name: {:?}", template_name);

            match registry.inner().get(&template_name) {
                Some(template) => {
                    trace!(target:"_", "Template found: {:?}", path);
                    return Outcome::Success(Self(*template));
                }
                None => {
                    warn!(target:"_", "Template not found: {:?}", template_name);
                    return Outcome::Forward(Status::NotFound);
                }
            }
        } else {
            return Outcome::Error((Status::InternalServerError, "Invalid path".to_string()));
        }
    }
}

#[get("/<_..>")]
pub fn template_routes(template: Templated) -> Template {
    Template::render(template.0, context! {})
}

fn remove_extension(mut path: PathBuf) -> PathBuf {
    let stem = path.file_stem().map(|s| s.to_string_lossy().to_string());
    match stem {
        None => path,
        Some(stem) => {
            match stem.split('.').next() {
                Some(stem) => path.set_file_name(stem),
                None => path.set_file_name(stem),
            }
            path
        }
    }
}
