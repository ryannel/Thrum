use crate::{Listener, Place, VoiceSettings};

pub(crate) fn place_valid(place: Option<Place>) -> bool {
    place_finite(place) && place.is_none_or(|p| p.radius > 0.0)
}

pub(crate) fn place_finite(place: Option<Place>) -> bool {
    place.is_none_or(|p| p.radius.is_finite() && finite(p.position))
}

pub(crate) fn settings_valid(settings: VoiceSettings) -> bool {
    settings_finite(settings) && settings.place.is_none_or(|p| p.radius > 0.0)
}

pub(crate) fn settings_finite(settings: VoiceSettings) -> bool {
    settings.volume.is_finite() && settings.pitch.is_finite() && place_finite(settings.place)
}

pub(crate) fn listener_finite(listener: Listener) -> bool {
    finite(listener.position) && finite(listener.right) && listener.near.is_finite()
}

pub(crate) fn normalized(mut listener: Listener) -> Option<Listener> {
    if !listener_finite(listener) || listener.near <= 0.0 {
        return None;
    }
    let right = listener.right.map(f64::from);
    let length = dot(right, right).sqrt();
    if length == 0.0 {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "normalized components fit in f32"
    )]
    {
        listener.right = right.map(|component| (component / length) as f32);
    }
    Some(listener)
}

fn finite(value: [f32; 3]) -> bool {
    value.into_iter().all(f32::is_finite)
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn offsets(place: Place, listener: Listener) -> ([f64; 3], f64) {
    let offset =
        std::array::from_fn(|i| f64::from(place.position[i]) - f64::from(listener.position[i]));
    (offset, dot(offset, offset))
}

fn attenuation(distance: f64, radius: f32) -> f32 {
    let remaining = 1.0 - distance / f64::from(radius);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "linear gain is within 0 to 1"
    )]
    let gain = (remaining * remaining) as f32;
    gain
}

pub(crate) fn loudness(settings: VoiceSettings, listener: Listener) -> Option<f32> {
    let Some(place) = settings.place else {
        return Some(settings.volume);
    };
    let (_, distance_squared) = offsets(place, listener);
    let distance = distance_squared.sqrt();
    (distance < f64::from(place.radius))
        .then(|| settings.volume * attenuation(distance, place.radius))
}

pub(crate) fn gains(settings: VoiceSettings, listener: Listener) -> [f32; 2] {
    let Some(place) = settings.place else {
        return [settings.volume; 2];
    };
    let (offset, distance_squared) = offsets(place, listener);
    let distance = distance_squared.sqrt();
    if distance >= f64::from(place.radius) {
        return [0.0; 2];
    }
    let near = f64::from(listener.near);
    let pan = (dot(offset, listener.right.map(f64::from))
        / (distance_squared + near * near).sqrt())
    .clamp(-1.0, 1.0);
    let gain = settings.volume * attenuation(distance, place.radius);
    #[expect(
        clippy::cast_possible_truncation,
        reason = "pan scales are within 0 to sqrt(2)"
    )]
    let pan_gains = [(1.0 - pan).sqrt() as f32, (1.0 + pan).sqrt() as f32];
    pan_gains.map(|scale| gain * scale)
}

pub(crate) fn glide(current: f32, target: f32, step: f32) -> f32 {
    current + (target - current).clamp(-step, step)
}
