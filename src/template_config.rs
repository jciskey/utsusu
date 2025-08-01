//! This module provides configuration specification and parsing for templates.

use std::fmt;
use std::fs::read_to_string;
use std::path::Path;
use std::collections::HashMap;
use saphyr::{LoadableYamlNode, YamlOwned, ScalarOwned};
use globset::{Glob, GlobSet};

// TODO:
// - config string parsing fn
// - config file parsing fn

const CONFIG_KEY_OUTPUT_TYPE: &str = "type";
const CONFIG_KEY_OUTPUT_TOP_LEVEL: &str = "output";
const CONFIG_KEY_OUTPUT_FILENAME: &str = "filename";
const CONFIG_KEY_OUTPUT_DIRECTORY: &str = "directory";
const CONFIG_KEY_INCLUDED_FILES: &str = "include";
const CONFIG_KEY_VARIABLES: &str = "variables";


/// Represents the different output types of a particular template
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemplateOutputType {
    /// The template renders a single file
    File,

    /// The template renders a directory tree
    Directory,
}

/// Contains the configuration for a particular template.
#[derive(Clone)]
pub struct TemplateConfig {

    /// The glob matching patterns for files that should be included in the rendered output
    included_file_patterns: GlobSet,

    /// This maps variable names to default values.
    variables: HashMap<String, String>,

    /// What this template outputs when it does rendering: a file, or a directory tree.
    output_type: TemplateOutputType,

    // Output::Filename: string; the default name of the file to write the rendered file template to
    /// The filename to render this template to, if the output type is [TemplateOutputType::File],
    /// otherwise None.
    output_filename: Option<String>,

    // Output::Directory: string; the default name of the directory to write the rendered directory template to
    /// The directory to render this template to, if the output type is
    /// [TemplateOutputType::Directory], otherwise None.
    output_directory: Option<String>,
}

impl TemplateConfig {
    /// Creates a new, empty template configuration.
    pub fn new() -> Self {
        Self {
            included_file_patterns: GlobSet::empty(),
            variables: HashMap::new(),
            output_type: TemplateOutputType::File,
            output_filename: None,
            output_directory: None,
        }
    }

    /// Adds a pattern to the list of matching patterns for files that this template will render.
    pub fn update_included_file_patterns(&mut self, globset: GlobSet) {
        self.included_file_patterns = globset;
    }

    /// Returns whether the given file should be rendered by this template.
    pub fn should_include_file<P: AsRef<Path>>(&self, path: &P) -> bool {
        self.included_file_patterns.is_match(path)
    }

    /// Returns clones of all the (key, default) variable pairs.
    pub fn get_variable_items(&self) -> Vec<(String, String)> {
        self.variables.iter().map(|(k,v)| (k.clone(), v.clone())).collect()
    }

    /// Adds or updates a variable to have a particular default value, which will be used for
    /// rendering if the invoker doesn't override it at render time.
    ///
    /// Returns the previous default value if one was set, None otherwise.
    pub fn add_variable(&mut self, variable_name: String, default: String) -> Option<String> {
        self.variables.insert(variable_name, default)
    }

    /// Updates the output type of the template. If the type is actually changed, this will also
    /// wipe the associated output name (filename or directory).
    pub fn set_output_type(&mut self, output_type: TemplateOutputType) {
        match (self.output_type, output_type) {
            (TemplateOutputType::File, TemplateOutputType::File) |
            (TemplateOutputType::Directory, TemplateOutputType::Directory) => {},
            (TemplateOutputType::File, _) => {
                self.output_type = output_type;
                self.output_filename = None;
            },
            (TemplateOutputType::Directory, _) => {
                self.output_type = output_type;
                self.output_directory = None;
            }
        }
    }

    pub fn get_output_type(&self) -> TemplateOutputType {
        self.output_type
    }

    /// Sets the default output name for a File output template. Does nothing if the output type is
    /// not [TemplateOutputType::File].
    pub fn set_output_filename(&mut self, filename: String) {
        if self.output_type == TemplateOutputType::File {
            self.output_filename = Some(filename);
        }
    }

    pub fn get_output_filename(&self) -> Option<&str> {
        self.output_filename.as_deref()
    }

    /// Sets the default output directory name for a Directory output template. Does nothing if the
    /// output type is not [TemplateOutputType::Directory].
    pub fn set_output_directory(&mut self, directory: String) {
        if self.output_type == TemplateOutputType::Directory {
            self.output_directory = Some(directory);
        }
    }

    pub fn get_output_directory(&self) -> Option<&str> {
        self.output_directory.as_deref()
    }

    pub fn get_render_context(&self) -> tera::Context {
        let mut context = tera::Context::new();

        for (k, v) in self.variables.iter() {
            context.insert(k, v);
        }

        context
    }
}

impl fmt::Debug for TemplateConfig {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("TemplateConfig")
		 .field("output_type", &self.output_type)
		 .field("output_filename", &self.output_filename)
		 .field("output_directory", &self.output_directory)
		 .field("variables", &self.variables)
		 .finish()
	}
}

#[derive(Debug, Clone)]
pub enum ConfigParseError {
    YamlParseError(saphyr::ScanError),
    ConfigMustBeAMapping,
    NoOutputConfig,
    OutputConfigMustBeAMapping,
    NoOutputType,
    InvalidOutputType,
    NoOutputFilename,
    InvalidOutputFilename,
    NoOutputDirectory,
    InvalidOutputDirectory,
    NoIncludedFiles,
    InvalidIncludedFiles,
    TooManyIncludedFileGlobs,
    IncludedFileGlobMustBeString,
    IncludedFileGlobParseError(Option<String>, globset::ErrorKind),
    VariablesMustBeAMapping,
    VariableNameMustBeAString,
    VariableDefaultMustBeAScalar,
}

pub fn parse_config_from_yaml_string(yaml: &str) -> Result<TemplateConfig, ConfigParseError> {

    // Load the YAML
    match YamlOwned::load_from_str(yaml) {
        Err(error) => {
            Err(ConfigParseError::YamlParseError(error))
        },
        Ok(docs) => {
            let config_doc = &docs[0];
            match config_doc {
                YamlOwned::Mapping(mapping) => {
                    let mut config = TemplateConfig::new();
                    // Parse the data
                    // - Output type
                    let output_type = match mapping.get(&YamlOwned::Value(ScalarOwned::String(CONFIG_KEY_OUTPUT_TYPE.to_string()))) {
                        Some(owned_val) => {
                            match owned_val {
                                YamlOwned::Value(ScalarOwned::String(val)) => {
                                    match val.as_str() {
                                        "file" => TemplateOutputType::File,
                                        "directory" => TemplateOutputType::Directory,
                                        _ => return Err(ConfigParseError::InvalidOutputType),
                                    }
                                },
                                _ => return Err(ConfigParseError::InvalidOutputType),
                            }
                        },
                        None => return Err(ConfigParseError::NoOutputType),
                    };

                    // - Output filename/directory
                    let output_mapping = match mapping.get(&YamlOwned::Value(ScalarOwned::String(CONFIG_KEY_OUTPUT_TOP_LEVEL.to_string()))) {
                        Some(owned_val) => {
                            match owned_val {
                                YamlOwned::Mapping(owned_mapping) => {
                                    owned_mapping
                                },
                                _ => return Err(ConfigParseError::OutputConfigMustBeAMapping),
                            }
                        },
                        None => return Err(ConfigParseError::NoOutputConfig),
                    };

                    match output_type {
                        TemplateOutputType::File => {
                            match output_mapping.get(&YamlOwned::Value(ScalarOwned::String(CONFIG_KEY_OUTPUT_FILENAME.to_string()))) {
                                Some(owned_val) => {
                                    match owned_val {
                                        YamlOwned::Value(ScalarOwned::String(val)) => {
                                            config.set_output_filename(val.clone());
                                        },
                                        _ => return Err(ConfigParseError::InvalidOutputFilename),
                                    }
                                },
                                None => return Err(ConfigParseError::NoOutputFilename),
                            };
                        },
                        TemplateOutputType::Directory => {
                            match output_mapping.get(&YamlOwned::Value(ScalarOwned::String(CONFIG_KEY_OUTPUT_DIRECTORY.to_string()))) {
                                Some(owned_val) => {
                                    match owned_val {
                                        YamlOwned::Value(ScalarOwned::String(val)) => {
                                            config.set_output_directory(val.clone());
                                        },
                                        _ => return Err(ConfigParseError::InvalidOutputDirectory),
                                    }
                                },
                                None => return Err(ConfigParseError::NoOutputDirectory),
                            };
                        },
                    };

                    // - Included files -- These entries are globs to be used for matching, not direct filenames
                    match mapping.get(&YamlOwned::Value(ScalarOwned::String(CONFIG_KEY_INCLUDED_FILES.to_string()))) {
                        Some(owned_val) => {
                            let mut file_globs = GlobSet::builder();
                            match owned_val {
                                YamlOwned::Value(ScalarOwned::String(val)) => {
                                    match Glob::new(val.as_str()) {
                                        Ok(glob) => file_globs.add(glob),
                                        Err(glob_err) => {
                                            let originating_glob = glob_err.glob().and_then(|s| Some(s.to_string()));
                                            return Err(ConfigParseError::IncludedFileGlobParseError(originating_glob, glob_err.kind().clone()));
                                        },
                                    };
                                },
                                YamlOwned::Sequence(seq) => {
                                    // If the output type is a File, then there should only be a
                                    // single filename glob provided
                                    if output_type == TemplateOutputType::File {
                                        if seq.len() > 1 {
                                            return Err(ConfigParseError::TooManyIncludedFileGlobs);
                                        }
                                    }

                                    for v in seq {
                                        match v {
                                            YamlOwned::Value(ScalarOwned::String(val)) => {
                                                match Glob::new(val.as_str()) {
                                                    Ok(glob) => file_globs.add(glob),
                                                    Err(glob_err) => {
                                                        let originating_glob = glob_err.glob().and_then(|s| Some(s.to_string()));
                                                        return Err(ConfigParseError::IncludedFileGlobParseError(originating_glob, glob_err.kind().clone()));
                                                    },
                                                };
                                            },
                                            _ => return Err(ConfigParseError::IncludedFileGlobMustBeString),
                                        };
                                    }
                                },
                                _ => return Err(ConfigParseError::InvalidIncludedFiles),
                            };
                            match file_globs.build() {
                                Ok(globset) => config.update_included_file_patterns(globset),
                                Err(glob_err) => {
                                    let originating_glob = glob_err.glob().and_then(|s| Some(s.to_string()));
                                    return Err(ConfigParseError::IncludedFileGlobParseError(originating_glob, glob_err.kind().clone()));
                                },
                            };
                        },
                        None => return Err(ConfigParseError::NoIncludedFiles),
                    };
                    

                    // - Variables
                    match mapping.get(&YamlOwned::Value(ScalarOwned::String(CONFIG_KEY_VARIABLES.to_string()))) {
                        Some(owned_val) => {
                            match owned_val {
                                YamlOwned::Mapping(variables_mapping) => {
                                    for (variable_name, variable_default_value) in variables_mapping.iter() {
                                        match variable_name {
                                            YamlOwned::Value(ScalarOwned::String(string_var_name)) => {
                                                match variable_default_value {
                                                    YamlOwned::Value(scalar_value) => {
                                                        let var_name_key = string_var_name.to_string();
                                                        match scalar_value {
                                                            ScalarOwned::Null => config.add_variable(var_name_key, "".to_string()),
                                                            ScalarOwned::Boolean(bool_default_value) => config.add_variable(var_name_key, bool_default_value.to_string()),
                                                            ScalarOwned::Integer(int_default_value) => config.add_variable(var_name_key, int_default_value.to_string()),
                                                            ScalarOwned::FloatingPoint(fp_default_value) => config.add_variable(var_name_key, fp_default_value.to_string()),
                                                            ScalarOwned::String(string_default_value) => config.add_variable(var_name_key, string_default_value.to_string()),
                                                        };
                                                    },
                                                    _ => return Err(ConfigParseError::VariableDefaultMustBeAScalar),
                                                };
                                            },
                                            _ => return Err(ConfigParseError::VariableNameMustBeAString),
                                        };
                                    }
                                },
                                _ => return Err(ConfigParseError::VariablesMustBeAMapping),
                            };
                        },
                        None => {},  // Do nothing, variables are not a required field
                    };

                    // All done, return the config
                    Ok(config)
                },
                _ => Err(ConfigParseError::ConfigMustBeAMapping),
            }
        },
    }
}

#[derive(Debug)]
pub enum ConfigParseFromFileError {
    FileReadError(std::io::Error),
    ParseError(ConfigParseError),
}

pub fn parse_config_from_file<P: AsRef<Path>>(path: &P) -> Result<TemplateConfig, ConfigParseFromFileError> {
    match read_to_string(path) {
        Err(read_error) => Err(ConfigParseFromFileError::FileReadError(read_error)),
        Ok(config_str) => {
            match parse_config_from_yaml_string(&config_str) {
                Err(parse_error) => Err(ConfigParseFromFileError::ParseError(parse_error)),
                Ok(config) => Ok(config),
            }
        }
    }
}

