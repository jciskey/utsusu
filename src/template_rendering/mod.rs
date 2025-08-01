use std::path::{Path, PathBuf};
use tera::Tera;

pub mod single_file_render;
mod directory_render;

pub fn get_all_template_filenames_from_directory<P: AsRef<Path>>(dir: &P) -> std::io::Result<Vec<PathBuf>> {
    let mut filenames = Vec::new();

    let mut dirs_to_traverse: Vec<PathBuf> = Vec::new();

    dirs_to_traverse.push(dir.as_ref().to_path_buf());

    while let Some(traverse_dir) = dirs_to_traverse.pop() {
        for entry in std::fs::read_dir(traverse_dir)? {
            let entry = entry?; // This can be an Err if there was a permissions issue in the path chain
			let path = entry.path();
            if path.is_dir() {
                dirs_to_traverse.push(path);
            } else {
                filenames.push(path);
                //if let Ok(buffer) = path.strip_prefix(dir) {
                //    filenames.push(buffer.to_path_buf());
                //}
            }
        }
    }

    Ok(filenames)
}

pub fn load_template_files_from_filenames<P: AsRef<Path>>(files: &[P]) -> tera::Result<Tera> {
    let mut tera = Tera::default();

    let _ = tera.add_template_files(
        files
            .iter()
            .filter(|p| {
                let p_ref = p.as_ref();
                p_ref.is_file()
            })
            .map(|p| (p, None::<String>))
    )?;

    Ok(tera)
}

