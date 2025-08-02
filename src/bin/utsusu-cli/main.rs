use std::env;
use std::process::exit;
use std::path::PathBuf;

use directories::ProjectDirs;
use clap::{Arg, Command};

use utsusu::utils::{get_user_input, get_user_variable_choices};
use utsusu::template_rendering::{load_template_files_from_filenames, get_all_template_filenames_from_directory};
use utsusu::template_rendering::single_file_render::render_single_file;
use utsusu::template_config::{parse_config_from_file, TemplateOutputType};

// CLI parsing:
// - Should be as simple as specifying the template name as a positional argument
// - Should allow overriding the configuration file via flag
// - Should allow specifying the template directory via flag

const DEFAULT_CONFIG_FILE: &str = "config.yml";
const DEFAULT_TEMPLATE_CONFIG_FILE: &str = "config.yml";
const DEFAULT_TEMPLATE_DIR: &str = "templates";
const TEMPLATE_FILES_DIR: &str = "files";

const CONFIG_FILE_PARAM_NAME: &str = "config_file";
const TEMPLATES_DIR_PARAM_NAME: &str = "templates_directory";
const TEMPLATE_NAME_PARAM_NAME: &str = "template_name";

const CONFIG_FILE_ENV_NAME: &str = "UTSUSU_CONFIG_FILE";
const TEMPLATES_DIR_ENV_NAME: &str = "UTSUSU_TEMPLATES_DIR";

pub fn main() {
    let project_dirs_opt = ProjectDirs::from("", "", "utsusu");

    let (default_config_file_path, default_template_dir_path) = match &project_dirs_opt {
        Some(project_dirs) => {
            let default_config_file_path = project_dirs.config_dir().join(DEFAULT_CONFIG_FILE).to_path_buf();
            let default_template_dir_path = project_dirs.data_dir().join(DEFAULT_TEMPLATE_DIR).to_path_buf();
            (default_config_file_path, default_template_dir_path)
        },
        None => (PathBuf::default(), PathBuf::default()),
    };

    let empty_pathbuf = PathBuf::default();

    let help_string_default_config_file_path = if default_config_file_path != empty_pathbuf {
        format!(" [default: {}]", default_config_file_path.display())
    } else { String::new() };

    let help_string_default_template_dir_path = if default_template_dir_path != empty_pathbuf {
        format!(" [default: {}]", default_template_dir_path.display())
    } else { String::new() };

    let cli = Command::new("utsusu")
        .version(env!("CARGO_PKG_VERSION"))
        .about("A straightforward template rendering binary")
        .arg(
            Arg::new(CONFIG_FILE_PARAM_NAME)
                .short('c')
				.long("config")
				.required(false)
                .env(CONFIG_FILE_ENV_NAME)
				.value_name("CONFIG_FILE")
				.help(format!("Path to the configuration file to use{}", help_string_default_config_file_path))
		)
        .arg(
            Arg::new(TEMPLATES_DIR_PARAM_NAME)
                .short('t')
				.long("templates-dir")
				.required(false)
                .env(TEMPLATES_DIR_ENV_NAME)
				.value_name("TEMPLATES_DIR")
				.help(format!("Path to the directory containing templates to render{}", help_string_default_template_dir_path))
		)
        .arg(
            Arg::new(TEMPLATE_NAME_PARAM_NAME)
                .required(true)
                .value_name("NAME")
                .help("The name of the template to render")
        );

    let matches = cli.get_matches();

    let utsusu_config_file_path = match matches.get_one::<String>(CONFIG_FILE_PARAM_NAME) {
        Some(path_str) => PathBuf::from(path_str),
        None => {
            // Fall-back to default config file path (if available)
            if let Some(project_dirs) = &project_dirs_opt {
                default_config_file_path
            } else {
                // Can't find the file, error and tell the user to explicitly specify the config
                // file path
                eprintln!("Cannot find default configuration file path, specify explicitly via the {} environment variable or via the flag {}", CONFIG_FILE_ENV_NAME, "--config");
                exit(1);
            }
        },
    };

    // TODO: Parse the provided config file to extract relevant info

    let templates_dir_path = match matches.get_one::<String>(TEMPLATES_DIR_PARAM_NAME) {
        Some(path_str) => PathBuf::from(path_str),
        None => {
            // TODO: If the user provided a config file, we can try reading that for the relevant data

            // Fall-back to default templates directory path (if available)
            if let Some(project_dirs) = project_dirs_opt {
                default_template_dir_path
            } else {
                // Can't find the templates directory path, error and tell the user to explicitly specify the templates
                // directory path
                eprintln!("Cannot find default templates directory path, specify explicitly via the {} environment variable or via the flag {}", TEMPLATES_DIR_ENV_NAME, "--templates-dir");
                exit(1);
            }
        },
    };

    // Construct the path to the requested template
    let requested_template_name = match matches.get_one::<String>(TEMPLATE_NAME_PARAM_NAME) {
        Some(path_str) => PathBuf::from(path_str),
        None => {
            // This should never happen, since this parameter is marked as required, and clap
            // checks for that already
            eprintln!("Fatal error determining requested template. This is a bug, please report it on the project Github.");
            exit(1);
        },
    };

    let requested_template_path = templates_dir_path.join(requested_template_name);
    println!("Template Path: {:?}", requested_template_path);

    // Validate that the template path exists
    if !requested_template_path.is_dir() {
        eprintln!("Template does not exist at path '{}'", requested_template_path.display());
        exit(1);
    }

    // Pull config file from template directory
    let template_config_file_path = requested_template_path.join(DEFAULT_TEMPLATE_CONFIG_FILE);
    println!("Using config file at: {}", template_config_file_path.display());

    let template_config_res = parse_config_from_file(&template_config_file_path);

    if template_config_res.is_err() {
        println!("Error parsing configuration: {:?}", template_config_res.unwrap_err());
        exit(-1);
    }

    let template_config = template_config_res.unwrap();

    // Aggregate the template files that should be rendered
    let mut template_files_to_render: Vec<PathBuf> = Vec::new();

    let template_files_path = requested_template_path.join(TEMPLATE_FILES_DIR);
    let res = get_all_template_filenames_from_directory(&template_files_path);
    if let Ok(files) = res {
        for f in files {
            if let Ok(files_dir_relative_filename) = f.strip_prefix(&template_files_path) {
                if template_config.should_include_file(&files_dir_relative_filename) {
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
        match template_config.get_output_type() {
            TemplateOutputType::File => {
                let output_opt = get_user_input(&format!("Output File [{}]: ", template_config.get_output_filename().unwrap_or("rendered")));
                (output_opt, None)
            },
            TemplateOutputType::Directory => {
                let output_opt = get_user_input(&format!("Output Directory [{}]: ", template_config.get_output_directory().unwrap_or("rendered")));
                (None, output_opt)
            },
        }
    };

    // -- Template variables
    let user_variables_context = get_user_variable_choices(&template_config);

    // Do the output rendering
    match template_config.get_output_type() {
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
                    let output_file_path = user_output_filename.or_else(|| template_config.get_output_filename().and_then(|s| Some(s.to_string()))).unwrap_or(String::new());
                    match render_single_file(&tera, &template_config, &template_source_file_path.display().to_string(), Some(&user_variables_context)) {
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
                                println!("Error writing rendered file: {}", write_res.unwrap_err());
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
        TemplateOutputType::Directory => {
            match load_template_files_from_filenames(&template_files_to_render) {
                Err(tera_error) => {
                    println!("Error loading template files: {}", tera_error);
                    exit(-4);
                },
                Ok(tera) => {
                    // Create the output directory
                    let output_directory_path = PathBuf::from(user_output_directory.or_else(|| template_config.get_output_directory().and_then(|s| Some(s.to_string()))).unwrap_or(String::new()));
                    if let Err(fs_error) = std::fs::create_dir(&output_directory_path) {
                        println!("Error creating output directory: {}", fs_error);
                        exit(-8);
                    }

                    // Render all the files to the output directory
                    let total_template_files = template_files_to_render.len();
                    let mut total_template_files_written = 0;
                    for template_source_file_path in &template_files_to_render {
                        if let Ok(files_dir_relative_filename) = template_source_file_path.strip_prefix(&template_files_path) {
                            let output_file_path = output_directory_path.join(files_dir_relative_filename);
                            match render_single_file(&tera, &template_config, &template_source_file_path.display().to_string(), Some(&user_variables_context)) {
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
                                        println!("Error writing rendered file: {}", write_res.unwrap_err());
                                        println!("Output file path: {}", output_file_path.display());
                                        exit(-7);
                                    } else {
                                        total_template_files_written += 1;
                                    }
                                },
                            };
                        }
                    }
                    println!("{}/{} files written to '{}'", total_template_files_written, total_template_files, output_directory_path.display());
                    exit(0);
                },
            };
        },
        //_ => println!("Unsupported output type: {:?}", template_config.get_output_type()),
    };

}
