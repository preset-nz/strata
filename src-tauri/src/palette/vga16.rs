use super::colour::{lab_delta_e, srgb_to_lab, Lab, Srgb};

pub struct Bucket {
    pub name: &'static str,
    pub srgb: Srgb,
}

pub const VGA16: &[Bucket] = &[
    Bucket {
        name: "black",
        srgb: Srgb {
            r: 0x00,
            g: 0x00,
            b: 0x00,
        },
    },
    Bucket {
        name: "maroon",
        srgb: Srgb {
            r: 0x80,
            g: 0x00,
            b: 0x00,
        },
    },
    Bucket {
        name: "red",
        srgb: Srgb {
            r: 0xFF,
            g: 0x00,
            b: 0x00,
        },
    },
    Bucket {
        name: "purple",
        srgb: Srgb {
            r: 0x80,
            g: 0x00,
            b: 0x80,
        },
    },
    Bucket {
        name: "fuchsia",
        srgb: Srgb {
            r: 0xFF,
            g: 0x00,
            b: 0xFF,
        },
    },
    Bucket {
        name: "green",
        srgb: Srgb {
            r: 0x00,
            g: 0x80,
            b: 0x00,
        },
    },
    Bucket {
        name: "lime",
        srgb: Srgb {
            r: 0x00,
            g: 0xFF,
            b: 0x00,
        },
    },
    Bucket {
        name: "olive",
        srgb: Srgb {
            r: 0x80,
            g: 0x80,
            b: 0x00,
        },
    },
    Bucket {
        name: "yellow",
        srgb: Srgb {
            r: 0xFF,
            g: 0xFF,
            b: 0x00,
        },
    },
    Bucket {
        name: "navy",
        srgb: Srgb {
            r: 0x00,
            g: 0x00,
            b: 0x80,
        },
    },
    Bucket {
        name: "blue",
        srgb: Srgb {
            r: 0x00,
            g: 0x00,
            b: 0xFF,
        },
    },
    Bucket {
        name: "teal",
        srgb: Srgb {
            r: 0x00,
            g: 0x80,
            b: 0x80,
        },
    },
    Bucket {
        name: "aqua",
        srgb: Srgb {
            r: 0x00,
            g: 0xFF,
            b: 0xFF,
        },
    },
    Bucket {
        name: "silver",
        srgb: Srgb {
            r: 0xC0,
            g: 0xC0,
            b: 0xC0,
        },
    },
    Bucket {
        name: "gray",
        srgb: Srgb {
            r: 0x80,
            g: 0x80,
            b: 0x80,
        },
    },
    Bucket {
        name: "white",
        srgb: Srgb {
            r: 0xFF,
            g: 0xFF,
            b: 0xFF,
        },
    },
];

pub fn nearest_bucket(lab: Lab) -> &'static Bucket {
    let mut best = &VGA16[0];
    let mut best_d = f32::INFINITY;
    for bucket in VGA16 {
        let d = lab_delta_e(lab, srgb_to_lab(bucket.srgb));
        if d < best_d {
            best_d = d;
            best = bucket;
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pure_red_maps_to_red() {
        let lab = srgb_to_lab(Srgb { r: 255, g: 0, b: 0 });
        assert_eq!(nearest_bucket(lab).name, "red");
    }

    #[test]
    fn pure_green_maps_to_lime() {
        let lab = srgb_to_lab(Srgb { r: 0, g: 255, b: 0 });
        assert_eq!(nearest_bucket(lab).name, "lime");
    }

    #[test]
    fn dark_red_maps_to_maroon() {
        let lab = srgb_to_lab(Srgb {
            r: 100,
            g: 10,
            b: 10,
        });
        assert_eq!(nearest_bucket(lab).name, "maroon");
    }

    #[test]
    fn mid_grey_maps_to_gray() {
        let lab = srgb_to_lab(Srgb {
            r: 128,
            g: 128,
            b: 128,
        });
        assert_eq!(nearest_bucket(lab).name, "gray");
    }
}
