use std::{fs, io::Read};

use common::result::HttpResult;

pub fn load_file(path: &str) -> HttpResult<String> {
    let mut file = fs::File::options().read(true).open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}
