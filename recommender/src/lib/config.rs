#[derive(Clone, Debug)]
pub struct Config {
    pub amp_url: String,
    pub region: String,
    pub context: Option<String>,
    pub namespace: Option<String>,
}

impl Config {
    pub fn new(
        amp_url: String,
        region: String,
        context: Option<String>,
        namespace: Option<String>,
    ) -> Self {
        Self {
            amp_url,
            region,
            context,
            namespace,
        }
    }
}
