use std::env;
use std::io;
use std::io::Write;
use std::process::exit;
use std::path::PathBuf;
use utsusu::template_rendering::{load_template_files_from_filenames, get_all_template_filenames_from_directory};
use utsusu::template_rendering::single_file_render::render_single_file;
use utsusu::template_config::{parse_config_from_file, parse_config_from_yaml_string, TemplateOutputType};

pub fn main() {
    // Pull template directory path from last CLI arg
    let args: Vec<String> = env::args().collect();
    let dir = (&args).last().unwrap();
    println!("Template Path: {:?}", dir);

    // TODO: Convert this into unit tests
    /*
    let config_str = "
    type: file
    output:
      filename: test.rs
    include: template.rs
    ";

    let config = parse_config_from_yaml_string(&config_str);
    println!("Config: {:?}", config);
    */

    // Pull config file from template directory
    let dir_path = PathBuf::from(dir);
    let config_file_path = dir_path.join("config.yml");
    println!("Using config file at: {}", config_file_path.display());

    let config_res = parse_config_from_file(&config_file_path);

    if config_res.is_err() {
        println!("Error parsing configuration: {:?}", config_res.unwrap_err());
        exit(-1);
    }

    let config = config_res.unwrap();

    let mut template_files_to_render: Vec<PathBuf> = Vec::new();

    let template_files_path = dir_path.join("files");
    let res = get_all_template_filenames_from_directory(&template_files_path);
    if let Ok(files) = res {
        for f in files {
            if let Ok(files_dir_relative_filename) = f.strip_prefix(&template_files_path) {
                if config.should_include_file(&files_dir_relative_filename) {
                    template_files_to_render.push(f);
                }
            }
        }
    } else {
        println!("Error reading template files: {}", res.unwrap_err());
        exit(-2);
    }

    if template_files_to_render.len() == 0 {
        println!("No matching template files to render. Adjust your included files glob to match at least one file.");
        exit(-5);
    }

    // Get user values for variables
    // -- Output filename/directory is always needed
    let (user_output_filename, user_output_directory) = {
        let mut input = String::new();

        match config.get_output_type() {
            TemplateOutputType::File => {
                print!("Output File [{}]: ", config.get_output_filename().unwrap_or("rendered"));
                io::stdout().flush();  // Ensure the message is displayed to the user before requesting input
                io::stdin().read_line(&mut input);
                let trimmed_input = input.trim();
                let output_opt = if trimmed_input == "" {
                    None
                } else {
                    Some(trimmed_input.to_string())
                };
                (output_opt, None)
            },
            TemplateOutputType::Directory => {
                print!("Output Directory [{}]: ", config.get_output_directory().unwrap_or("rendered"));
                io::stdout().flush();  // Ensure the message is displayed to the user before requesting input
                io::stdin().read_line(&mut input);
                let trimmed_input = input.trim();
                let output_opt = if trimmed_input == "" {
                    None
                } else {
                    Some(trimmed_input.to_string())
                };
                (None, output_opt)
            },
        }
    };

    // -- Loop through the variables they defined in the config file
    let mut user_variables_context: tera::Context = tera::Context::new();
    for (var_name, default_var_value) in config.get_variable_items() {
        let mut input = String::new();

        print!("{} [{}]: ", var_name, default_var_value);
        io::stdout().flush();  // Ensure the message is displayed to the user before requesting input
        io::stdin().read_line(&mut input);
        let trimmed_input = input.trim();
        if trimmed_input != "" {
            user_variables_context.insert(var_name, trimmed_input);
        };
    }

    match config.get_output_type() {
        TemplateOutputType::File => {
            if template_files_to_render.len() > 1 {
                println!("Cannot render more than 1 file for a 'File' type template. Adjust your included files glob to match a single file.");
                exit(-3);
            }

            match load_template_files_from_filenames(&template_files_to_render) {
                Err(tera_error) => {
                    println!("Error loading template files: {}", tera_error);
                    exit(-4);
                },
                Ok(tera) => {
                    let template_source_file_path = &template_files_to_render[0]; // Safety: Due to previous checks, this will always have exactly 1 element
                    let output_file_path = user_output_filename.or_else(|| config.get_output_filename().and_then(|s| Some(s.to_string()))).unwrap_or(String::new());
                    match render_single_file(&tera, &config, &template_source_file_path.display().to_string(), Some(&user_variables_context)) {
                        Err(tera_error) => {
                            println!("Error rendering template file: {}", tera_error);
                            println!("Source file: {}", template_source_file_path.display());
                            println!("All template files: {:?}", template_files_to_render);
                            println!("Registered templates: {:?}", tera.get_template_names().collect::<Vec<_>>());
                            exit(-6);
                        },
                        Ok(rendered_string) => {
                            // Write the rendered string to the output file
                            let write_res = std::fs::write(&output_file_path, rendered_string);
                            if write_res.is_err() {
                                println!("Error rendering template file: {}", write_res.unwrap_err());
                                exit(-7);
                            } else {
                                println!("Template written to '{}'", output_file_path);
                                exit(0);
                            }
                        },
                    };
                },
            };
        },
        _ => println!("Unsupported output type: {:?}", config.get_output_type()),
    };

}
