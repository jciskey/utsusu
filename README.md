# utsusu

```
写す (utsusu) -- Japanese verb, "to copy; to duplicate; to reproduce"
```

[Jisho.org](https://jisho.org/word/%E5%86%99%E3%81%99)

utsusu is a simple library (and associated CLI tool) for copying templates into a final location. These templates can be either a full directory, or a single file, whichever makes sense for your needs.

## Goals

utsusu intends to be a flexible tool, giving you the benefits of a full directory-templating system while also allowing you to use that same system for stamping out individual files.

Specific use-cases:

- Single CLI command to stamp out an individual file from a template.
  - Examples:
    - A Nomad jobspec for your specific cluster setup
    - A standalone single-file script with standard headers and dependencies (see `examples/rust-script` for an example of this)
- Single CLI command to stamp out a full directory from a template.
  - Examples:
    - A new project template with a README, license file, dependencies, and baseline code tree
    - A template for creating new utsusu templates (see `examples/utsusu-template` for an example of this)
- Library availability for programmatically generating templates from your own code
  - Cookiecutter has gotten better about this, but for a long time programmatically using it was nearly impossible, making full automation not as easy as one would like.
  - In general, if you want to incorporate templating into your own code, you have to do everything yourself, building on a baseline template rendering engine.

## Similar Projects

- Cookiecutter is a Python tool that does this for directories.
- `stamp-cli` is a Rust CLI tool that also does this for directories.
- Spawn Point is a Rust CLI tool that also does this for directories.
- Hugo (the static site generator) provides the ability to template out individual pages according to the type of content it is, but does not allow you to generate subdirectories via template.

The first three are focused on directories, and do not let you stamp out individual files separate from a directory. To do that, you generally have to directly invoke a templating library such as Jinja2, Tera, or something else. The fourth is focused on letting you easily generate new individual pages, but doesn't allow you to generate a directory from a template if you have a standard format for different sections of your site (how much of a pain this is is entirely up to the site you're building).

## Usage

To see full usage instructions of the CLI, run `utsusu --help`.

Example usage:

```bash
utsusu --templates-dir examples utsusu-template
```

Sample configurations can be found in the examples directory. The `utsusu-template` example will produce a simple single-file template that you can work off of to get started.
