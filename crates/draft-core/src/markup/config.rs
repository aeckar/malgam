/// Dynamic configuration options set by the `\file` macro or by `config.mgon`.
///
/// These options can be changed at any point within a markup file by a macro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynConf {
    pub latex_math: bool,  // `latex` todo
    pub code_lang: String, // `code` todo
}

impl DynConf {
    /// Returns a new instance with the following configuration:
    /// ```toml
    /// latex_math = false
    /// code_lang = "txt"
    /// ```
    fn default() -> Self {
        Self {
            latex_math: false,
            code_lang: "txt".to_string(),
        }
    }
}

/// Static configuration options set using compiler flags or by `config.mgon`.
///
/// These options cannot be changed from within a markup file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticConf {
    /// If true, does not recognize inline math formatting to make writing finances easier.
    pub finance_mode: bool,

    /// If true, does not perform a first pass to ensure the input is valid UTF-8.
    /// 
    /// # Safety
    /// This should only be enabled in local environments, where the user can be trusted
    /// to pass valid input to the compiler.
    pub trusted_mode: bool,

    /// If true, recognizes links without having to use link syntax.
    pub infer_links: bool,

    /// If true, paragraph spacing is always 1 (every line is the start of a unique element).
    /// 
    /// This should be enabled in environments where paragraphs and list elements wrap to the next
    /// line, and are seperated by a newline.
    pub single_line_mode: bool,
}

impl StaticConf {
    /// Returns a new instance with the following configuration:
    /// ```toml
    /// finance_mode = false
    /// trusted_mode = false
    /// infer_links = true
    /// wrap_mode = false
    /// ```
    fn default() -> Self {
        Self {
            finance_mode: false,
            trusted_mode: false,
            infer_links: true,
            single_line_mode: false,
        }
    }
}
