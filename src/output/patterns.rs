use std::collections::BTreeMap;
use std::fmt::{self, Write};

use heck::ToShoutySnakeCase;
use regex::Regex;
use serde_json::json;

use super::formatter::Formatter;
use super::CodeWriter;

// Helper function to sanitize names for various languages
fn sanitize_identifier(name: &str, for_rust_constant: bool) -> String {
    // Replace common problematic characters with underscores
    let mut sanitized = name.replace(|c: char| !c.is_alphanumeric() && c != '_', "_");

    // If it's for a Rust constant, convert to UPPER_SNAKE_CASE
    if for_rust_constant {
        sanitized = sanitized.to_shouty_snake_case();
    }

    // Ensure it doesn't start with a digit (common requirement)
    if sanitized.chars().next().map_or(false, |c| c.is_digit(10)) {
        sanitized = format!("_{}", sanitized);
    }

    // Replace multiple underscores with a single one
    let re = Regex::new(r"_+").unwrap();
    sanitized = re.replace_all(&sanitized, "_").into_owned();

    // Remove leading/trailing underscores that might result from replacements if not desired
    // For this use case, leading underscores are fine (e.g. if original started with digit)
    // Trailing underscores might be undesirable if they are not meaningful.
    if sanitized.ends_with('_') && sanitized.len() > 1 { // Avoid turning "_" into ""
        // Check if the original name also ended with a special char, if so, keep the underscore
        if !name.ends_with(|c: char| !c.is_alphanumeric()) {
             // Only trim if the original didn't justify an underscore at the end
            let mut chars = sanitized.chars();
            chars.next_back();
            sanitized = chars.as_str().to_string();
        }
    }


    if sanitized.is_empty() {
        return "_empty_".to_string();
    }

    sanitized
}

impl CodeWriter for BTreeMap<String, BTreeMap<String, String>> {
    fn write_cs(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        writeln!(fmt, "namespace CS2Dumper.Patterns;")?;
        fmt.empty_line()?;

        fmt.block("public static class GlobalPatterns", false, |fmt| {
            for (module_name, patterns) in self {
                let sanitized_module_name = sanitize_identifier(module_name, false);
                fmt.block(
                    &format!("public static class {}", sanitized_module_name),
                    false,
                    |fmt| {
                        for (pattern_name, pattern_value) in patterns {
                            let sanitized_pattern_name = sanitize_identifier(pattern_name, false);
                            // Use verbatim string literals for C#
                            writeln!(
                                fmt,
                                "public const string {} = @\"{}\";",
                                sanitized_pattern_name, pattern_value
                            )?;
                        }
                        Ok(())
                    },
                )?;
                fmt.empty_line()?;
            }
            Ok(())
        })?;
        Ok(())
    }

    fn write_hpp(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        writeln!(fmt, "#pragma once")?;
        // While <cstdint> was in the prompt, it's not strictly necessary for char* constants.
        // #include <cstdint> might be needed if we were defining integer types.
        // For char*, no specific include is essential beyond standard library availability.
        // For this specific case, <string_view> might be more modern if using C++17,
        // but constexpr const char* is fine and universally compatible.
        fmt.empty_line()?;

        fmt.block("namespace CS2Dumper::Patterns", true, |fmt| {
            for (module_name, patterns) in self {
                let sanitized_module_name = sanitize_identifier(module_name, false);
                fmt.block(
                    &format!("namespace {}", sanitized_module_name),
                    true,
                    |fmt| {
                        for (pattern_name, pattern_value) in patterns {
                            let sanitized_pattern_name = sanitize_identifier(pattern_name, false);
                            // Use C++ raw string literals R"-(...)-"
                            writeln!(
                                fmt,
                                "constexpr const char* {} = R\"-({})-\";",
                                sanitized_pattern_name, pattern_value
                            )?;
                        }
                        Ok(())
                    },
                )?;
                if module_name != self.keys().last().unwrap() { // Add empty line between module namespaces
                    fmt.empty_line()?;
                }
            }
            Ok(())
        })?;
        Ok(())
    }

    fn write_json(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        let json_value = json!(self);
        // The formatter is designed for line-by-line writing,
        // so directly using it for pre-formatted JSON might not be ideal.
        // However, if fmt.write_str is available and works, it's fine.
        // Or, ensure the Formatter's internal writer is used.
        fmt.write_str(&serde_json::to_string_pretty(&json_value).unwrap())?;
        Ok(())
    }

    fn write_rs(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        fmt.block("pub mod cs2_dumper_patterns", false, |fmt| {
            for (module_name, patterns) in self {
                let sanitized_module_name = sanitize_identifier(module_name, true).to_lowercase(); // Rust modules are snake_case
                fmt.block(&format!("pub mod {}", sanitized_module_name), false, |fmt| {
                    for (pattern_name, pattern_value) in patterns {
                        // For Rust constants, names are UPPER_SNAKE_CASE
                        let sanitized_pattern_name = sanitize_identifier(pattern_name, true);
                        // Use Rust raw string literals r#"..."#
                        writeln!(
                            fmt,
                            "pub const {}: &str = r#\"{}\"#;",
                            sanitized_pattern_name, pattern_value
                        )?;
                    }
                    Ok(())
                })?;
                if module_name != self.keys().last().unwrap() { // Add empty line between modules
                     fmt.empty_line()?;
                }
            }
            Ok(())
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::formatter::Formatter; // Adjusted path
    use std::collections::BTreeMap;

    fn get_test_patterns() -> BTreeMap<String, BTreeMap<String, String>> {
        let mut patterns = BTreeMap::new();

        let mut client_patterns = BTreeMap::new();
        client_patterns.insert("dwEntityList".to_string(), "pattern_entity_list".to_string());
        client_patterns.insert("dwLocalPlayer".to_string(), "pattern_local_player".to_string());
        client_patterns.insert("player.health".to_string(), "pattern_player_health".to_string());


        let mut engine_patterns = BTreeMap::new();
        engine_patterns.insert("dwViewMatrix".to_string(), "pattern_view_matrix".to_string());
        engine_patterns.insert("123_TestPattern".to_string(), "pattern_123_test".to_string());
        engine_patterns.insert("another-pattern!".to_string(), "pattern_another_test".to_string());

        patterns.insert("client.dll".to_string(), client_patterns);
        patterns.insert("engine-2.dll".to_string(), engine_patterns);

        patterns
    }

    #[test]
    fn test_write_cs_patterns() {
        let patterns = get_test_patterns();
        let mut buffer = String::new();
        let mut fmt = Formatter::new(&mut buffer, 4);

        patterns.write_cs(&mut fmt).unwrap();

        let expected_output = r#"namespace CS2Dumper.Patterns;

public static class GlobalPatterns {
    public static class client_dll {
        public const string dwEntityList = @"pattern_entity_list";
        public const string dwLocalPlayer = @"pattern_local_player";
        public const string player_health = @"pattern_player_health";
    }

    public static class engine_2_dll {
        public const string dwViewMatrix = @"pattern_view_matrix";
        public const string _123_TestPattern = @"pattern_123_test";
        public const string another_pattern = @"pattern_another_test";
    }

}
"#;
        assert_eq!(buffer, expected_output);
    }

    #[test]
    fn test_write_hpp_patterns() {
        let patterns = get_test_patterns();
        let mut buffer = String::new();
        let mut fmt = Formatter::new(&mut buffer, 4); // Assuming indent_size is 4 for consistency

        patterns.write_hpp(&mut fmt).unwrap();

        // Note: The logic for fmt.block adds a trailing newline if the block is not empty.
        // And empty_line adds one too. The last module doesn't get an empty_line after it.
        let expected_output = r#"#pragma once

namespace CS2Dumper::Patterns {
    namespace client_dll {
        constexpr const char* dwEntityList = R"-(pattern_entity_list)-";
        constexpr const char* dwLocalPlayer = R"-(pattern_local_player)-";
        constexpr const char* player_health = R"-(pattern_player_health)-";
    }

    namespace engine_2_dll {
        constexpr const char* dwViewMatrix = R"-(pattern_view_matrix)-";
        constexpr const char* _123_TestPattern = R"-(pattern_123_test)-";
        constexpr const char* another_pattern = R"-(pattern_another_test)-";
    }
}
"#;
        assert_eq!(buffer, expected_output);
    }

    #[test]
    fn test_write_rs_patterns() {
        let patterns = get_test_patterns();
        let mut buffer = String::new();
        let mut fmt = Formatter::new(&mut buffer, 4);

        patterns.write_rs(&mut fmt).unwrap();

        let expected_output = r#"pub mod cs2_dumper_patterns {
    pub mod client_dll {
        pub const DW_ENTITY_LIST: &str = r#"pattern_entity_list"#;
        pub const DW_LOCAL_PLAYER: &str = r#"pattern_local_player"#;
        pub const PLAYER_HEALTH: &str = r#"pattern_player_health"#;
    }

    pub mod engine_2_dll {
        pub const DW_VIEW_MATRIX: &str = r#"pattern_view_matrix"#;
        pub const _123_TEST_PATTERN: &str = r#"pattern_123_test"#;
        pub const ANOTHER_PATTERN: &str = r#"pattern_another_test"#;
    }
}
"#;
        assert_eq!(buffer, expected_output);
    }
     #[test]
    fn test_sanitize_identifier_various_cases() {
        assert_eq!(sanitize_identifier("dwEntityList", false), "dwEntityList");
        assert_eq!(sanitize_identifier("dwEntityList", true), "DW_ENTITY_LIST");
        assert_eq!(sanitize_identifier("client.dll", false), "client_dll");
        assert_eq!(sanitize_identifier("client.dll", true), "CLIENT_DLL");
        assert_eq!(sanitize_identifier("engine-2.dll", false), "engine_2_dll");
        assert_eq!(sanitize_identifier("engine-2.dll", true), "ENGINE_2_DLL");
        assert_eq!(sanitize_identifier("123_TestPattern", false), "_123_TestPattern");
        assert_eq!(sanitize_identifier("123_TestPattern", true), "_123_TEST_PATTERN");
        assert_eq!(sanitize_identifier("player.health", false), "player_health");
        assert_eq!(sanitize_identifier("player.health", true), "PLAYER_HEALTH");
        assert_eq!(sanitize_identifier("another-pattern!", false), "another_pattern"); // Trailing '!' removed
        assert_eq!(sanitize_identifier("another-pattern!", true), "ANOTHER_PATTERN");
        assert_eq!(sanitize_identifier("with_!§$%&/()=?", false), "with"); // Only keeps 'with'
        assert_eq!(sanitize_identifier("with_!§$%&/()=?", true), "WITH");
        assert_eq!(sanitize_identifier("_leading_underscore", false), "_leading_underscore");
        assert_eq!(sanitize_identifier("_leading_underscore", true), "_LEADING_UNDERSCORE");
        assert_eq!(sanitize_identifier("trailing_underscore_", false), "trailing_underscore_");
        assert_eq!(sanitize_identifier("trailing_underscore_", true), "TRAILING_UNDERSCORE_");
        assert_eq!(sanitize_identifier("multiple__underscores", false), "multiple_underscores");
        assert_eq!(sanitize_identifier("multiple__underscores", true), "MULTIPLE_UNDERSCORES");
        assert_eq!(sanitize_identifier("", false), "_empty_");
        assert_eq!(sanitize_identifier("---", false), "_");
        assert_eq!(sanitize_identifier("---", true), "_");
    }
}
