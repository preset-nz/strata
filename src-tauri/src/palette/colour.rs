#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Srgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Copy, Clone, Debug)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

const XN: f32 = 0.95047;
const YN: f32 = 1.00000;
const ZN: f32 = 1.08883;

pub fn srgb_to_lab(c: Srgb) -> Lab {
    let r = srgb_to_linear(c.r as f32 / 255.0);
    let g = srgb_to_linear(c.g as f32 / 255.0);
    let b = srgb_to_linear(c.b as f32 / 255.0);

    let x = 0.4124564 * r + 0.3575761 * g + 0.1804375 * b;
    let y = 0.2126729 * r + 0.7151522 * g + 0.0721750 * b;
    let z = 0.0193339 * r + 0.119_192 * g + 0.9503041 * b;

    let fx = lab_f(x / XN);
    let fy = lab_f(y / YN);
    let fz = lab_f(z / ZN);

    Lab {
        l: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

pub fn lab_to_srgb(l: Lab) -> Srgb {
    let fy = (l.l + 16.0) / 116.0;
    let fx = fy + l.a / 500.0;
    let fz = fy - l.b / 200.0;

    let xr = lab_f_inv(fx);
    let yr = lab_f_inv(fy);
    let zr = lab_f_inv(fz);

    let x = xr * XN;
    let y = yr * YN;
    let z = zr * ZN;

    let r = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
    let g = -0.969_266 * x + 1.8760108 * y + 0.0415560 * z;
    let b = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;

    Srgb {
        r: linear_to_srgb_u8(r),
        g: linear_to_srgb_u8(g),
        b: linear_to_srgb_u8(b),
    }
}

pub fn lab_delta_e(a: Lab, b: Lab) -> f32 {
    let dl = a.l - b.l;
    let da = a.a - b.a;
    let db = a.b - b.b;
    (dl * dl + da * da + db * db).sqrt()
}

pub fn lab_chroma(l: Lab) -> f32 {
    (l.a * l.a + l.b * l.b).sqrt()
}

pub fn lab_hue_deg(l: Lab) -> f32 {
    let h = l.b.atan2(l.a).to_degrees();
    if h < 0.0 {
        h + 360.0
    } else {
        h
    }
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb_u8(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let c = if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (c * 255.0).round() as u8
}

fn lab_f(t: f32) -> f32 {
    const D: f32 = 6.0 / 29.0;
    if t > D * D * D {
        t.powf(1.0 / 3.0)
    } else {
        t / (3.0 * D * D) + 4.0 / 29.0
    }
}

fn lab_f_inv(t: f32) -> f32 {
    const D: f32 = 6.0 / 29.0;
    if t > D {
        t * t * t
    } else {
        3.0 * D * D * (t - 4.0 / 29.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_roundtrip_pure_colours() {
        for c in [
            Srgb { r: 255, g: 0, b: 0 },
            Srgb { r: 0, g: 255, b: 0 },
            Srgb { r: 0, g: 0, b: 255 },
            Srgb {
                r: 128,
                g: 128,
                b: 128,
            },
            Srgb {
                r: 255,
                g: 255,
                b: 255,
            },
            Srgb { r: 0, g: 0, b: 0 },
        ] {
            let lab = srgb_to_lab(c);
            let back = lab_to_srgb(lab);
            let dr = (back.r as i16 - c.r as i16).abs();
            let dg = (back.g as i16 - c.g as i16).abs();
            let db = (back.b as i16 - c.b as i16).abs();
            assert!(dr <= 1 && dg <= 1 && db <= 1, "{:?} -> {:?}", c, back);
        }
    }

    #[test]
    fn pure_red_lab_within_known_range() {
        let lab = srgb_to_lab(Srgb { r: 255, g: 0, b: 0 });
        assert!((lab.l - 53.24).abs() < 0.1);
        assert!(lab_chroma(lab) > 100.0);
    }
}
