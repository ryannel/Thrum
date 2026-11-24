//! The one parse of the settings that choose the water scene.
//!
//! The window passes its `THRUM_WATER_*` environment variables to [`Settings::from_variables`], the page
//! its query to [`Settings::from_query`], and the proof tests their environment the same way, so a proof
//! builds the scene a person opens. The library reads no environment itself. The proof harness's own
//! variables, which say where a proof writes and what it records, are not parsed here.
use crate::floating::{WakeDemo, WakeView, WhitewaterDemo};
use crate::island::Wind;

/// Each setting's page query name and environment variable.
const FOAM: (&str, &str) = ("foam", "THRUM_WATER_FOAM");
const LOCATION: (&str, &str) = ("location", "THRUM_WATER_LOCATION");
const WAKE: (&str, &str) = ("wake", "THRUM_WATER_WAKE");
const BODIES: (&str, &str) = ("bodies", "THRUM_WATER_BODIES");
const SPEED: (&str, &str) = ("speed", "THRUM_WATER_SPEED");
const HEADING: (&str, &str) = ("heading", "THRUM_WATER_HEADING");
const VIEW: (&str, &str) = ("view", "THRUM_WATER_VIEW");
const STONE: (&str, &str) = ("stone", "THRUM_WATER_STONE");
const WAKE_BODIES: (&str, &str) = ("wake-bodies", "THRUM_WATER_WAKE_BODIES");
const ISLAND: (&str, &str) = ("island", "THRUM_WATER_ISLAND");
const SURF: (&str, &str) = ("surf", "THRUM_WATER_SURF");

/// The scene the settings choose.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Settings {
    /// The open trade sea, chosen when no other setting names a scene.
    OpenSea,
    /// The island under a wind, with transported whitewater on its beach when `foam` is set.
    Island {
        /// Weather the island is loaded with.
        wind: Wind,
        /// Whitewater on the shore, chosen by `foam` with `location=beach`.
        foam: bool,
    },
    /// Island beach with refraction of the long waves and the breaker section.
    Surf(Wind),
    /// The craft and buoys, with or without the wake route.
    Floating(Floating),
}

/// The floating-body scene's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Floating {
    /// Weather the bodies float in.
    pub wind: Wind,
    /// The inland lake rather than the sea, from `location=lake`.
    pub lake: bool,
    /// Initial craft speed in metres per second, 0 to 15.
    pub speed: f32,
    /// Initial craft heading in radians: beam (π/2) from `heading=beam`, otherwise π.
    pub heading: f32,
    /// The chase camera, from `view=chase`.
    pub chase: bool,
    /// The wake route, chosen by `wake` or by `foam` away from the beach.
    pub wake: Option<Wake>,
}

/// The wake route's settings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wake {
    /// Inspection camera, from `view`.
    pub view: WakeView,
    /// A stone dropped at one second, only when `stone=1`.
    pub stone: bool,
    /// Admitted body reactions for a capacity inspection, from `wake-bodies`.
    pub body_capacity: Option<u32>,
    /// Transported whitewater, on whenever `foam` is given.
    pub whitewater: bool,
}

impl Wake {
    /// The wake demonstration the window builds, with whitewater cooked when `cooked` is set.
    pub fn demo(self, cooked: bool) -> WakeDemo {
        WakeDemo {
            view: self.view,
            stone: self.stone,
            body_capacity: self.body_capacity,
            whitewater: match (self.whitewater, cooked) {
                (false, _) => WhitewaterDemo::Off,
                (true, false) => WhitewaterDemo::Procedural,
                (true, true) => WhitewaterDemo::Cooked,
            },
            ..WakeDemo::default()
        }
    }
}

impl Settings {
    /// Parse the settings from `THRUM_WATER_*` variables, given a lookup of a variable's value by name,
    /// such as the process environment.
    pub fn from_variables(variable: impl Fn(&str) -> Option<String>) -> Self {
        Self::parse(|(_, name)| variable(name))
    }

    /// Read the settings from a page query such as `?wake=calm&view=overhead`.
    pub fn from_query(query: &str) -> Self {
        Self::parse(|(key, _)| {
            query
                .trim_start_matches('?')
                .split('&')
                .filter_map(|parameter| parameter.split_once('='))
                .find_map(|(name, value)| (name == key).then(|| value.to_owned()))
        })
    }

    fn parse(get: impl Fn((&str, &str)) -> Option<String>) -> Self {
        if let Some(wind) = get(SURF).as_deref().and_then(Wind::named) {
            return Self::Surf(wind);
        }
        let foam = get(FOAM);
        let location = get(LOCATION);
        let beach = location.as_deref() == Some("beach");
        if beach && let Some(wind) = foam.as_deref().and_then(Wind::named) {
            return Self::Island { wind, foam: true };
        }
        let wake = get(WAKE).or_else(|| foam.clone().filter(|_| !beach));
        let floating = wake.clone().or_else(|| get(BODIES));
        if let Some(wind) = floating.as_deref().and_then(Wind::named) {
            let view = get(VIEW);
            return Self::Floating(Floating {
                wind,
                lake: location.as_deref() == Some("lake"),
                speed: get(SPEED)
                    .and_then(|value| value.parse::<f32>().ok())
                    .unwrap_or(0.0)
                    .clamp(0.0, 15.0),
                heading: if get(HEADING).as_deref() == Some("beam") {
                    std::f32::consts::FRAC_PI_2
                } else {
                    std::f32::consts::PI
                },
                chase: view.as_deref() == Some("chase"),
                wake: wake.map(|_| Wake {
                    view: WakeView::named(view.as_deref().unwrap_or_default()),
                    stone: get(STONE).as_deref() == Some("1"),
                    body_capacity: get(WAKE_BODIES).and_then(|value| value.parse().ok()),
                    whitewater: foam.is_some(),
                }),
            });
        }
        match get(ISLAND).as_deref().and_then(Wind::named) {
            Some(wind) => Self::Island { wind, foam: false },
            None => Self::OpenSea,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn floating(query: &str) -> Floating {
        match Settings::from_query(query) {
            Settings::Floating(floating) => floating,
            other => panic!("{query} chose {other:?}"),
        }
    }

    #[test]
    fn no_setting_opens_the_sea() {
        assert_eq!(Settings::from_query(""), Settings::OpenSea);
        assert_eq!(Settings::from_query("?island=storm"), Settings::OpenSea);
    }

    #[test]
    fn island_and_beach() {
        let island = Settings::Island {
            wind: Wind::Trade,
            foam: false,
        };
        assert_eq!(Settings::from_query("?island=trade"), island);
        // The beach without foam falls through to the island preset.
        assert_eq!(Settings::from_query("?island=trade&location=beach"), island);
        // The beach with foam comes before every other scene, the wake among them.
        assert_eq!(
            Settings::from_query("?foam=gale&location=beach&wake=calm&island=trade"),
            Settings::Island {
                wind: Wind::Gale,
                foam: true
            }
        );
    }

    #[test]
    fn floating_reads_speed_heading_view_and_lake() {
        let plain = floating("?bodies=gale");
        assert_eq!(plain.wind, Wind::Gale);
        assert_eq!((plain.speed, plain.heading), (0.0, PI));
        assert!(!plain.lake && !plain.chase && plain.wake.is_none());
        let set = floating("?bodies=trade&speed=40&heading=beam&view=chase&location=lake");
        assert_eq!((set.speed, set.heading), (15.0, FRAC_PI_2));
        assert!(set.lake && set.chase);
        // A chase view alone keeps the default heading.
        assert_eq!(floating("?bodies=trade&view=chase").heading, PI);
        assert_eq!(floating("?bodies=calm&speed=fast").speed, 0.0);
    }

    #[test]
    fn wake_reads_its_demonstration() {
        let wake = floating("?wake=calm&view=shore&stone=1&wake-bodies=0&heading=beam")
            .wake
            .expect("wake route");
        assert_eq!(
            wake,
            Wake {
                view: WakeView::Shore,
                stone: true,
                body_capacity: Some(0),
                whitewater: false,
            }
        );
        assert_eq!(floating("?wake=calm&heading=beam").heading, FRAC_PI_2);
        // The stone is on only when it is `1`.
        assert!(!floating("?wake=calm&stone=true").wake.expect("wake").stone);
        assert!(!floating("?wake=calm&stone=0").wake.expect("wake").stone);
    }

    #[test]
    fn foam_away_from_the_beach_is_the_wake_route_with_whitewater() {
        let sea = floating("?foam=gale&view=vista");
        assert_eq!(sea.wind, Wind::Gale);
        let wake = sea.wake.expect("wake route");
        assert!(wake.whitewater);
        assert_eq!(wake.view, WakeView::Vista);
        assert_eq!(wake.demo(true).whitewater, WhitewaterDemo::Cooked);
        assert_eq!(wake.demo(false).whitewater, WhitewaterDemo::Procedural);
        // An explicit wake wind comes before the foam's.
        let both = floating("?foam=gale&wake=calm");
        assert_eq!(both.wind, Wind::Calm);
        assert!(both.wake.expect("wake").whitewater);
        assert_eq!(
            floating("?wake=calm")
                .wake
                .expect("wake")
                .demo(true)
                .whitewater,
            WhitewaterDemo::Off
        );
    }

    #[test]
    fn an_unknown_wake_wind_shadows_the_bodies() {
        // The window took the first named route and only then interpreted it.
        assert_eq!(
            Settings::from_query("?wake=breeze&bodies=gale&island=calm"),
            Settings::Island {
                wind: Wind::Calm,
                foam: false
            }
        );
    }
}
