mod format;
mod strings;

const YAML_FILE_PATH: &'static str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../resources/strings.yml");

#[proc_macro]
pub fn make_string_library(_tokens: proc_macro::TokenStream) -> proc_macro::TokenStream {
    crate::strings::generate_library_from_yaml(YAML_FILE_PATH).into()
}
