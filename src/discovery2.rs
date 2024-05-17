use log::{error, info, trace, warn};
use rocket::{
    http::{Method, Status},
    response::Responder,
    route::{Handler, Outcome},
    Data, Request, Route,
};
use rocket_dyn_templates::{context, Template};
use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::Result,
    path::PathBuf,
};

#[derive(Debug)]
pub struct TemplateRegistry(pub HashMap<String, &'static str>);

impl TemplateRegistry {
    pub fn new(root: PathBuf) -> Result<&'static Self> {
        println!("Loading templates from: {:?}", root);
        let mut templates = HashMap::new();
        let mut queue = VecDeque::from(vec![root.clone()]);
        while !queue.is_empty() {
            let path = queue.pop_front().unwrap();
            if path.is_file() {
                println!("\t>> Found template: {:?}", path);
                let mut template_name = path.strip_prefix(&root).unwrap().to_path_buf();
                while template_name
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains('.')
                {
                    let name = template_name
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string();
                    template_name.set_file_name(name.split('.').next().unwrap());
                }
                let static_str: &'static str =
                    Box::leak(template_name.to_str().unwrap().to_string().into_boxed_str());
                println!("\t>> Loaded template: {}", static_str);
                templates.insert(static_str.to_string(), static_str);
            } else if path.is_dir() {
                fs::read_dir(path)?
                    .filter_map(|result| result.ok())
                    .map(|dir_entry| dir_entry.path())
                    .for_each(|path| queue.push_back(path))
            }
        }

        println!("Loaded {} templates", templates.len());
        Ok(Box::leak(Box::new(TemplateRegistry(templates))))
    }
}

#[derive(Debug, Clone)]
pub struct TemplateFileServer {
    rank: isize,
    use_index_files: bool,
    template_registry: Option<&'static TemplateRegistry>,
    custom_template_page_sub_root: Option<PathBuf>,
    public_root: PathBuf,
}

impl TemplateFileServer {
    pub fn builder() -> TemplateFileServerBuilder {
        TemplateFileServerBuilder(Self::default())
    }
}

#[rocket::async_trait]
impl Handler for TemplateFileServer {
    #[allow(unused_variables)]
    async fn handle<'r>(&self, req: &'r Request<'_>, data: Data<'r>) -> Outcome<'r> {
        // WARN: Find out the need for error handling
        let mut segments: PathBuf = req.segments(0..).unwrap();

        trace!(target:"_", "Segments: {:?}", segments);

        if segments.parent().is_none() && self.use_index_files {
            trace!(target:"_", "Using index files");
            segments.push("index");
        }

        if let Some(registry) = self.template_registry {
            // println!("Checking template registry");
            if let Some(path) = segments.to_str() {
                let template_name = match &self.custom_template_page_sub_root {
                    Some(template_page_root) => {
                        let template_name = PathBuf::from(path);
                        template_page_root
                            .join(template_name)
                            .to_str()
                            .unwrap()
                            .to_string()
                    }
                    None => path.to_string(),
                };
                trace!(target:"_", "Requested template name: {:?}", template_name);
                match registry.0.get(&template_name) {
                    Some(template) => {
                        trace!(target:"_", "Template found: {:?}", path);
                        if let Ok(res) = Template::render(*template, context!()).respond_to(req) {
                            info!(target:"_", "Responding with template: {:?}", template);
                            return Outcome::Success(res);
                        }
                        error!(target:"_", "Failed to render template: {:?}\npath:{:?}", template, path);
                    }
                    None => {
                        warn!(target:"_", "Template not found: {:?}", template_name);
                    }
                }
            }
        }

        // TODO: Add a FileServer inside the Template File Server to default to if no template is found
        error!("TemplateFileServer is not yet implemented");
        Outcome::forward(data, Status::NotFound)
    }
}

impl Default for TemplateFileServer {
    fn default() -> Self {
        Self {
            rank: 10,
            use_index_files: false,
            template_registry: None,
            custom_template_page_sub_root: None,
            public_root: PathBuf::from("public"),
        }
    }
}

impl Into<Vec<Route>> for TemplateFileServer {
    fn into(self) -> Vec<Route> {
        let name = format!(
            "TemplateFileServer: - templates: {:?}, public: {:?}",
            self.custom_template_page_sub_root.clone(),
            self.public_root.clone()
        )
        .into();
        let mut route = Route::ranked(self.rank, Method::Get, "/<path..>", self);
        route.name = Some(name);
        vec![route]
    }
}

pub struct TemplateFileServerBuilder(TemplateFileServer);

impl TemplateFileServerBuilder {
    pub fn rank(mut self, rank: isize) -> Self {
        self.0.rank = rank;
        self
    }

    pub fn use_index_files(mut self, use_index_files: bool) -> Self {
        self.0.use_index_files = use_index_files;
        self
    }

    pub fn template_registry(mut self, template_registry: &'static TemplateRegistry) -> Self {
        self.0.template_registry = Some(template_registry);
        self
    }

    pub fn template_page_root(mut self, template_page_root: Option<PathBuf>) -> Self {
        self.0.custom_template_page_sub_root = template_page_root;
        self
    }

    pub fn public_root(mut self, public_root: PathBuf) -> Self {
        self.0.public_root = public_root;
        self
    }

    pub fn generate_template_registry(mut self, root: PathBuf) -> Result<Self> {
        match TemplateRegistry::new(root) {
            Ok(registry) => {
                self.0.template_registry = Some(registry);
                Ok(self)
            }
            Err(err) => Err(err),
        }
    }

    pub fn build(self) -> TemplateFileServer {
        self.0
    }
}
