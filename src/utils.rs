use std::path::Path;

pub fn load_shader_source(shaders_root: &Path, name: &str) -> Result<String, std::io::Error> {
    let path = std::path::Path::new(shaders_root).join(name);
    let src = std::fs::read_to_string(path)?
        .lines()
        .map(|line| {
            if line.starts_with("//!include") {
                let path = line
                    .split_whitespace()
                    .nth(1)
                    .expect("invalid include statement")
                    .replace('"', "");
                load_shader_source(&Path::new(shaders_root).join("include"), &path)
            } else {
                Ok(line.to_owned())
            }
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");

    Ok(src)
}

