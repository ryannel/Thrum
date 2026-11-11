//! Generation-level recipe admission. Display facts stay with the platform.

/// Prepared hardware recipe. Portable remains the default until measured promotion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Recipe {
    /// Ordinary stored wgpu attachments.
    Portable,
    /// Graph-qualified transient attachments on unified-memory Metal.
    AppleTile,
}

/// A diagnostic request could not be admitted before preparation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecipeError {
    /// Expected `portable` or `apple-tile`.
    InvalidOverride,
    /// Required backend, unified memory or transient storage support is absent/unknown.
    UnsupportedAppleTile,
}

pub(super) fn resolve(
    backend: wgpu::Backend,
    unified: Option<bool>,
    transient: Option<bool>,
    requested: Option<&str>,
) -> Result<Recipe, RecipeError> {
    match requested {
        None | Some("portable") => Ok(Recipe::Portable),
        Some("apple-tile")
            if backend == wgpu::Backend::Metal
                && unified == Some(true)
                && transient == Some(true) =>
        {
            Ok(Recipe::AppleTile)
        }
        Some("apple-tile") => Err(RecipeError::UnsupportedAppleTile),
        Some(_) => Err(RecipeError::InvalidOverride),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_requires_known_backend_facts_and_explicit_request() {
        for backend in [
            wgpu::Backend::Metal,
            wgpu::Backend::BrowserWebGpu,
            wgpu::Backend::Vulkan,
        ] {
            for unified in [None, Some(false), Some(true)] {
                for transient in [None, Some(false), Some(true)] {
                    assert_eq!(
                        resolve(backend, unified, transient, None),
                        Ok(Recipe::Portable)
                    );
                    assert_eq!(
                        resolve(backend, unified, transient, Some("portable")),
                        Ok(Recipe::Portable)
                    );
                    let expected = if backend == wgpu::Backend::Metal
                        && unified == Some(true)
                        && transient == Some(true)
                    {
                        Ok(Recipe::AppleTile)
                    } else {
                        Err(RecipeError::UnsupportedAppleTile)
                    };
                    assert_eq!(
                        resolve(backend, unified, transient, Some("apple-tile")),
                        expected
                    );
                    assert_eq!(
                        resolve(backend, unified, transient, Some("invalid")),
                        Err(RecipeError::InvalidOverride)
                    );
                }
            }
        }
    }
}
