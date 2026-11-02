//! Portable path identity for a candidate source tree.

use std::collections::HashMap;
use unicase::UniCase;
use unicode_normalization::is_nfc;

/// One validated, relative, NFC source file path using `/` separators.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourcePath(String);

/// A path cannot identify one file consistently across the selected targets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathError {
    /// The path was empty, absolute, or contained an empty, `.` or `..` component.
    NotRelativeFile,
    /// The spelling was not in Unicode NFC.
    NotNfc,
    /// A component has syntax reserved by at least one selected target.
    NotPortable,
    /// Two file or directory names have conflicting full Unicode case-folded spellings.
    CaseFoldCollision,
    /// A file path is also an ancestor of another file path.
    FileAncestor,
}

impl SourcePath {
    /// Validate a portable file path without changing its exact spelling.
    pub fn parse(path: &str) -> Result<Self, PathError> {
        if path.is_empty() || path.starts_with('/') || path.ends_with('/') || path.contains("//") {
            return Err(PathError::NotRelativeFile);
        }
        if !is_nfc(path) {
            return Err(PathError::NotNfc);
        }
        for component in path.split('/') {
            if component == "." || component == ".." {
                return Err(PathError::NotRelativeFile);
            }
            if component.ends_with(['.', ' '])
                || component.chars().any(|ch| {
                    ch.is_control() || matches!(ch, '\\' | '<' | '>' | ':' | '"' | '|' | '?' | '*')
                })
                || is_windows_device_name(component)
            {
                return Err(PathError::NotPortable);
            }
        }
        Ok(Self(path.to_owned()))
    }

    /// The exact spelling used by source manifests and staging.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn is_windows_device_name(component: &str) -> bool {
    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .trim_end_matches(' ');
    if ["con", "prn", "aux", "nul", "conin$", "conout$"]
        .iter()
        .any(|name| stem.eq_ignore_ascii_case(name))
    {
        return true;
    }
    let Some(prefix) = stem.get(..3) else {
        return false;
    };
    if !prefix.eq_ignore_ascii_case("COM") && !prefix.eq_ignore_ascii_case("LPT") {
        return false;
    }
    let suffix = &stem[3..];
    matches!(
        suffix,
        "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
    )
}

/// Reject aliases and file/directory conflicts across a declared tree.
/// The caller retains the original paths; a temporary folded index and borrowed
/// directory-prefix map are allocated once, outside a game callback.
pub fn validate_paths(paths: &[SourcePath]) -> Result<(), PathError> {
    let mut folded: Vec<String> = paths
        .iter()
        .map(|path| UniCase::new(path.as_str()).to_folded_case())
        .collect();
    {
        let mut original_directories = HashMap::<&str, &str>::new();
        for (path, folded_path) in paths.iter().zip(&folded) {
            for ((original_end, _), (folded_end, _)) in path
                .as_str()
                .match_indices('/')
                .zip(folded_path.match_indices('/'))
            {
                let original = &path.as_str()[..original_end];
                let normalized = &folded_path[..folded_end];
                if let Some(previous) = original_directories.insert(normalized, original)
                    && previous != original
                {
                    return Err(PathError::CaseFoldCollision);
                }
            }
        }
    }
    folded.sort_unstable();
    for pair in folded.windows(2) {
        if pair[0] == pair[1] {
            return Err(PathError::CaseFoldCollision);
        }
    }
    for path in &folded {
        for (index, _) in path.match_indices('/') {
            if folded
                .binary_search_by(|candidate| candidate.as_str().cmp(&path[..index]))
                .is_ok()
            {
                return Err(PathError::FileAncestor);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_paths_reject_aliases_and_unsafe_target_names() {
        for path in ["", "/scene.json", "a//b", "a/../b", "a/./b"] {
            assert_eq!(SourcePath::parse(path), Err(PathError::NotRelativeFile));
        }
        for path in [
            "scene\\a",
            "CON.txt",
            "NUL .txt",
            "COM1 .json",
            "CONIN$",
            "CONOUT$.log",
            "lpt¹.json",
            "a:b",
            "a?.json",
            "a.",
            "a ",
        ] {
            assert_eq!(SourcePath::parse(path), Err(PathError::NotPortable));
        }
        assert_eq!(
            SourcePath::parse("cafe\u{301}.json"),
            Err(PathError::NotNfc)
        );
        let paths =
            ["scenes/Maße.json", "scenes/MASSE.json"].map(|path| SourcePath::parse(path).unwrap());
        assert_eq!(validate_paths(&paths), Err(PathError::CaseFoldCollision));
        let paths = ["A", "a/child.json"].map(|path| SourcePath::parse(path).unwrap());
        assert_eq!(validate_paths(&paths), Err(PathError::FileAncestor));
        let paths = ["Assets/a.bin", "assets/b.bin"].map(|path| SourcePath::parse(path).unwrap());
        assert_eq!(validate_paths(&paths), Err(PathError::CaseFoldCollision));
        let paths = ["a", "a-b", "a/b"].map(|path| SourcePath::parse(path).unwrap());
        assert_eq!(validate_paths(&paths), Err(PathError::FileAncestor));
        let paths =
            ["scenes/café.json", "src/game.rs"].map(|path| SourcePath::parse(path).unwrap());
        assert_eq!(validate_paths(&paths), Ok(()));
    }
}
