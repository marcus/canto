//! Canto's colors. Dark, warm-neutral surfaces with a magenta→amber accent.

use gpui_kit::{Background, Hsla, hsla, linear_color_stop, linear_gradient, rgb};

pub fn bg() -> Hsla { rgb(0x0c0c0f).into() }
pub fn panel() -> Hsla { rgb(0x131317).into() }
pub fn raised() -> Hsla { rgb(0x1b1b21).into() }
pub fn hover() -> Hsla { rgb(0x23232b).into() }
pub fn field() -> Hsla { rgb(0x17171c).into() }
pub fn line() -> Hsla { rgb(0x26262e).into() }
pub fn text() -> Hsla { rgb(0xf2f0ed).into() }
pub fn text_dim() -> Hsla { rgb(0xa4a1ab).into() }
pub fn text_faint() -> Hsla { rgb(0x6c6973).into() }
pub fn accent() -> Hsla { rgb(0xff4f8b).into() }
pub fn accent_2() -> Hsla { rgb(0xffa24c).into() }
pub fn danger() -> Hsla { rgb(0xff6b6b).into() }
pub fn success() -> Hsla { rgb(0x5ed6a0).into() }

/// The signature gradient on primary actions and progress.
pub fn accent_gradient() -> Background {
    linear_gradient(100., linear_color_stop(accent(), 0.), linear_color_stop(accent_2(), 1.))
}

/// Deterministic cover art: two hues derived from the song id.
pub fn cover_gradient(seed: &str) -> Background {
    let mut h = seed.bytes().fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619));
    // FNV's low bits barely move for similar ids; finish with murmur3's mixer.
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    let hue = (h % 360) as f32 / 360.0;
    let shift = 0.08 + ((h >> 9) % 18) as f32 / 100.0;
    let a = hsla(hue, 0.72, 0.56, 1.0);
    let b = hsla((hue + shift) % 1.0, 0.80, 0.30, 1.0);
    linear_gradient(135., linear_color_stop(a, 0.), linear_color_stop(b, 1.))
}
