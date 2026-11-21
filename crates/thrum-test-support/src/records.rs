//! Hold a crate's WGSL records to its Rust structs: member names, offsets, sizes and scalar kinds.
//!
//! [`bound`] checks every struct a shader binds as a uniform or storage buffer, and every struct those hold, so
//! a new bound struct without a Rust record fails. [`named`] checks named structs whether or not anything binds
//! them, for a record declared in a shader that does not parse alone. [`declarations`] keeps only a source's
//! structs and bindings, for a shader whose functions need another crate's library. A Rust record is a
//! [`RecordLayout`], from the renderer, or from [`record!`](crate::record) in a test that can see the type.

use std::collections::BTreeSet;

pub use thrum_render::{RecordLayout, RecordMember, RecordScalar};
use wgpu::naga::{self, Handle, Module, ScalarKind, Type, TypeInner, proc::Layouter};

/// The scalar kind a Rust member type stores, for [`record!`](crate::record). A nested record has none.
pub trait Kind {
    /// The scalar kind, or `None` for a nested record.
    const KIND: Option<RecordScalar>;
}

impl Kind for u32 {
    const KIND: Option<RecordScalar> = Some(RecordScalar::Uint);
}
impl Kind for i32 {
    const KIND: Option<RecordScalar> = Some(RecordScalar::Sint);
}
impl Kind for f32 {
    const KIND: Option<RecordScalar> = Some(RecordScalar::Float);
}
impl<T: Kind, const N: usize> Kind for [T; N] {
    const KIND: Option<RecordScalar> = T::KIND;
}

/// One member of a record, for [`record!`](crate::record): its size and kind come from the member's type.
#[doc(hidden)]
#[must_use]
pub fn member<T, F: Kind>(name: &'static str, offset: usize, _: fn(&T) -> &F) -> RecordMember {
    RecordMember {
        name,
        offset,
        size: size_of::<F>(),
        kind: F::KIND,
    }
}

/// The [`RecordLayout`] of a `#[repr(C)]` Rust struct, named as its WGSL struct, listing every member in WGSL
/// order. A member the WGSL names otherwise takes `as "name"`.
///
/// ```
/// #[repr(C)]
/// struct Material {
///     base_rgba: [f32; 4],
///     flags: u32,
/// }
/// let layout = thrum_test_support::record!(Material as "MaterialRecord", base_rgba as "base", flags);
/// assert_eq!((layout.name, layout.size), ("MaterialRecord", 20));
/// assert_eq!((layout.members[1].name, layout.members[1].offset), ("flags", 16));
/// ```
#[macro_export]
macro_rules! record {
    ($ty:ty $(as $name:literal)?, $($field:ident $(as $alias:literal)?),+ $(,)?) => {
        $crate::records::RecordLayout {
            name: $crate::record!(@name $ty $(, $name)?),
            size: ::std::mem::size_of::<$ty>(),
            members: vec![$(
                $crate::records::member(
                    $crate::record!(@field $field $(, $alias)?),
                    ::std::mem::offset_of!($ty, $field),
                    |record: &$ty| &record.$field,
                )
            ),+],
        }
    };
    (@name $ty:ty, $name:literal) => { $name };
    (@name $ty:ty) => { stringify!($ty) };
    (@field $field:ident, $alias:literal) => { $alias };
    (@field $field:ident) => { stringify!($field) };
}

/// Keep only `source`'s struct declarations and its `@group` bindings, for a shader whose functions need
/// declarations another crate supplies.
///
/// # Panics
/// Fails if a struct or a binding does not close.
#[must_use]
pub fn declarations(source: &str) -> String {
    let mut result = String::new();
    for marker in ["struct ", "@group("] {
        let mut rest = source;
        while let Some(at) = rest.find(marker) {
            rest = &rest[at..];
            let end = if marker == "struct " {
                rest.find('}').expect("struct closes") + 1
            } else {
                rest.find(';').expect("binding ends") + 1
            };
            result.push_str(&rest[..end]);
            result.push('\n');
            rest = &rest[end..];
        }
    }
    result
}

/// Parse WGSL, failing the test with the parser's report under `name`.
///
/// # Panics
/// Fails if `source` does not parse.
#[must_use]
pub fn parse(name: &str, source: &str) -> Module {
    naga::front::wgsl::parse_str(source)
        .unwrap_or_else(|err| panic!("{name}: {}", err.emit_to_string(source)))
}

/// Every struct `module` binds as a uniform or storage buffer, and every struct those hold.
#[must_use]
pub fn bound_structs(module: &Module) -> BTreeSet<Handle<Type>> {
    fn visit(module: &Module, ty: Handle<Type>, found: &mut BTreeSet<Handle<Type>>) {
        match &module.types[ty].inner {
            TypeInner::Struct { members, .. } => {
                if found.insert(ty) {
                    for member in members {
                        visit(module, member.ty, found);
                    }
                }
            }
            TypeInner::Array { base, .. } | TypeInner::BindingArray { base, .. } => {
                visit(module, *base, found);
            }
            _ => {}
        }
    }
    let mut found = BTreeSet::new();
    for (_, global) in module.global_variables.iter() {
        if global.binding.is_some()
            && matches!(
                global.space,
                naga::AddressSpace::Uniform | naga::AddressSpace::Storage { .. }
            )
        {
            visit(module, global.ty, &mut found);
        }
    }
    found
}

/// Check every struct `source` binds, and every struct those hold, against the record of the same name in
/// `layouts`, and return the names checked. `name` labels the shader in a failure.
///
/// # Panics
/// Fails if `source` does not parse, a bound struct has no record, or any member's name, offset, size or scalar
/// kind, the member count or the struct's size differs.
#[must_use]
pub fn bound<'a>(name: &str, source: &str, layouts: &'a [RecordLayout]) -> BTreeSet<&'a str> {
    let module = parse(name, source);
    let layouter = layouter(&module);
    let mut checked = BTreeSet::new();
    for ty in bound_structs(&module) {
        let record = module.types[ty].name.as_deref().expect("bound struct name");
        let rust = layouts
            .iter()
            .find(|layout| layout.name == record)
            .unwrap_or_else(|| panic!("bound WGSL struct {record} has no Rust Pod record"));
        compare(&module, &layouter, ty, rust);
        checked.insert(rust.name);
    }
    checked
}

/// Check the struct named by each of `layouts` in `source`, bound or not. `name` labels the shader in a failure.
///
/// # Panics
/// Fails if `source` does not parse, a record's struct is missing, or any member's name, offset, size or scalar
/// kind, the member count or the struct's size differs.
pub fn named(name: &str, source: &str, layouts: &[RecordLayout]) {
    let module = parse(name, source);
    let layouter = layouter(&module);
    for rust in layouts {
        let (ty, _) = module
            .types
            .iter()
            .find(|(_, ty)| ty.name.as_deref() == Some(rust.name))
            .unwrap_or_else(|| panic!("{name} declares no WGSL struct {}", rust.name));
        compare(&module, &layouter, ty, rust);
    }
}

fn layouter(module: &Module) -> Layouter {
    let mut layouter = Layouter::default();
    layouter.update(module.to_ctx()).expect("WGSL type layout");
    layouter
}

fn compare(module: &Module, layouter: &Layouter, ty: Handle<Type>, rust: &RecordLayout) {
    let name = rust.name;
    let TypeInner::Struct { members, span } = &module.types[ty].inner else {
        panic!("{name} is not a WGSL struct")
    };
    assert_eq!(*span as usize, rust.size, "{name} size");
    assert_eq!(members.len(), rust.members.len(), "{name} member count");
    for (wgsl, rust) in members.iter().zip(&rust.members) {
        let member = rust.name;
        assert_eq!(wgsl.name.as_deref(), Some(member), "{name} member name");
        assert_eq!(wgsl.offset as usize, rust.offset, "{name}.{member} offset");
        assert_eq!(
            layouter[wgsl.ty].size as usize, rust.size,
            "{name}.{member} size"
        );
        assert_eq!(
            scalar_kind(module, wgsl.ty),
            rust.kind,
            "{name}.{member} scalar kind"
        );
    }
}

fn scalar_kind(module: &Module, ty: Handle<Type>) -> Option<RecordScalar> {
    match module.types[ty].inner {
        TypeInner::Scalar(scalar)
        | TypeInner::Atomic(scalar)
        | TypeInner::Vector { scalar, .. }
        | TypeInner::Matrix { scalar, .. } => Some(match scalar.kind {
            ScalarKind::Float => RecordScalar::Float,
            ScalarKind::Sint => RecordScalar::Sint,
            ScalarKind::Uint => RecordScalar::Uint,
            other => panic!("unexpected scalar kind {other:?}"),
        }),
        TypeInner::Array { base, .. } => scalar_kind(module, base),
        TypeInner::Struct { .. } => None,
        ref unexpected => panic!("unexpected member type {unexpected:?}"),
    }
}
