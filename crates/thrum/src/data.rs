use std::fmt::Debug;

use thrum_core::profile::{Digest, StateDecode, StateDecodeError, StateEncode, StateReader};

/// A value with a deterministic world representation.
/// ```
/// use thrum::Data;
/// #[derive(Data)]
/// struct Points(u32);
/// assert_eq!(Points(4), Points(4).clone());
/// ```
#[diagnostic::on_unimplemented(
    message = "T0101: `{Self}` cannot be held in Thrum world data",
    label = "this field has no portable world representation",
    note = "use a fixed-width integer, Num, or a bounded value built from Data fields"
)]
pub trait Data: Clone + Debug + PartialEq + Send + Sync + 'static {
    /// Stable identity of the type and its field declaration.
    /// ```
    /// use thrum::Data;
    /// assert_ne!(u32::SCHEMA, i32::SCHEMA);
    /// ```
    #[doc(hidden)]
    const SCHEMA: &'static str;

    /// Write canonical field values using the SDK's derive machinery.
    /// ```
    /// use thrum::Data;
    /// #[derive(Data)]
    /// struct Value(u16);
    /// assert_eq!(Value(7).0, 7);
    /// ```
    #[doc(hidden)]
    fn write(&self, writer: &mut DataWriter<'_, '_>);

    /// Read canonical field values using the SDK's derive machinery.
    ///
    /// # Errors
    ///
    /// The encoded fields are incomplete or invalid.
    /// ```
    /// use thrum::Data;
    /// #[derive(Data)]
    /// struct Value(u16);
    /// assert_eq!(Value(7).0, 7);
    /// ```
    #[doc(hidden)]
    fn read(reader: &mut DataReader<'_, '_>) -> Result<Self, InvalidData>;
}

macro_rules! marker {
    ($name:ident, $doc:literal, $example:literal) => {
        #[doc = $doc]
        #[doc = $example]
        pub trait $name: Data {}
    };
}

/// Copyable data attached to an entity.
/// ```
/// #[derive(thrum::Component)]
/// struct Health(u32);
/// let health = Health(7);
/// let copy = health;
/// assert_eq!(health, copy);
/// ```
pub trait Component: Data + Copy {}

marker!(
    Resource,
    "Data shared by rules in a game or scene.",
    r"
```
use thrum::Resource;
#[derive(Resource)]
struct Score(u32);
```
"
);
marker!(
    Event,
    "Data passed to later rules of the same tick.",
    r"
```
use thrum::Event;
#[derive(Event)]
struct Scored(u32);
```
"
);
marker!(
    SceneType,
    "The data a scene is entered with.",
    r"
```
use thrum::Scene;
#[derive(Scene, Default)]
struct Play;
```
"
);

/// Canonical writer passed to a game-data derive.
/// ```
/// #[derive(thrum::Data)]
/// struct Points(u32);
/// ```
pub struct DataWriter<'a, 'storage>(pub(crate) &'a mut Digest<'storage>);

/// Canonical reader passed to a game-data derive.
/// ```
/// #[derive(thrum::Data)]
/// struct Points(u32);
/// ```
pub struct DataReader<'a, 'storage>(pub(crate) &'a mut StateReader<'storage>);

/// A malformed value encountered by a game-data derive.
/// ```
/// use thrum::__derive::InvalidData;
/// assert_eq!(format!("{InvalidData:?}"), "InvalidData");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct InvalidData;

macro_rules! profile_data {
    ($($ty:ty),+ $(,)?) => {
        $(impl Data for $ty {
            const SCHEMA: &'static str = stringify!($ty);

            fn write(&self, writer: &mut DataWriter<'_, '_>) {
                self.encode(writer.0);
            }

            fn read(reader: &mut DataReader<'_, '_>) -> Result<Self, InvalidData> {
                Self::decode(reader.0).map_err(|_| InvalidData)
            }
        })+
    };
}

profile_data!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, bool);

impl Data for () {
    const SCHEMA: &'static str = "()";
    fn write(&self, _: &mut DataWriter<'_, '_>) {}
    fn read(_: &mut DataReader<'_, '_>) -> Result<Self, InvalidData> {
        Ok(())
    }
}

impl SceneType for () {}

impl<T: Data> Data for Option<T> {
    const SCHEMA: &'static str = "Option";

    fn write(&self, writer: &mut DataWriter<'_, '_>) {
        self.is_some().write(writer);
        if let Some(value) = self {
            value.write(writer);
        }
    }

    fn read(reader: &mut DataReader<'_, '_>) -> Result<Self, InvalidData> {
        bool::read(reader)?.then(|| T::read(reader)).transpose()
    }
}

impl<T: Data, const N: usize> Data for [T; N] {
    const SCHEMA: &'static str = "array";

    fn write(&self, writer: &mut DataWriter<'_, '_>) {
        for item in self {
            item.write(writer);
        }
    }

    fn read(reader: &mut DataReader<'_, '_>) -> Result<Self, InvalidData> {
        let mut items = [const { None }; N];
        for item in &mut items {
            *item = Some(T::read(reader)?);
        }
        Ok(items.map(|item| item.expect("all array items decoded")))
    }
}

macro_rules! tuple_data {
    ($($($ty:ident $index:tt),+);+ $(;)?) => {$(
        impl<$($ty: Data),+> Data for ($($ty,)+) {
            const SCHEMA: &'static str = stringify!(($($ty,)+));

            fn write(&self, writer: &mut DataWriter<'_, '_>) {
                $(self.$index.write(writer);)+
            }

            fn read(reader: &mut DataReader<'_, '_>) -> Result<Self, InvalidData> {
                Ok(($($ty::read(reader)?,)+))
            }
        }
    )+};
}

tuple_data!(
    A 0;
    A 0, B 1;
    A 0, B 1, C 2;
    A 0, B 1, C 2, D 3;
    A 0, B 1, C 2, D 3, E 4;
    A 0, B 1, C 2, D 3, E 4, F 5;
    A 0, B 1, C 2, D 3, E 4, F 5, G 6;
    A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7;
    A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8;
    A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9;
    A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10;
    A 0, B 1, C 2, D 3, E 4, F 5, G 6, H 7, I 8, J 9, K 10, L 11;
);

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Stored<T>(pub(crate) T);

impl<T: Data> StateEncode for Stored<T> {
    fn encode(&self, digest: &mut Digest<'_>) {
        self.0.write(&mut DataWriter(digest));
    }
}

impl<T: Data> StateDecode for Stored<T> {
    fn decode(reader: &mut StateReader<'_>) -> Result<Self, StateDecodeError> {
        T::read(&mut DataReader(reader))
            .map(Self)
            .map_err(|_| StateDecodeError::Invalid)
    }
}

marker!(
    PhaseType,
    "Data naming one phase of a scene.",
    r"
```
#[derive(thrum::Phase)]
enum Stage { Play, Finished }
```
"
);
