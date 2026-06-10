use image::{DynamicImage, Rgba, RgbaImage};

/// Per-region adaptive lighting normalization (CLAHE).
///
/// This is the second half of the MediaPipe "detect-then-track" idea: because
/// the landmark model only ever sees a *tight crop* of one body part (the face,
/// or a single hand), we can normalize the lighting INSIDE that crop on its own
/// — without ever touching the camera source. Each call to `predict()` enhances
/// its own region independently, so the face and each hand each get their own
/// exposure/contrast correction tuned to their local pixels.
///
/// We use CLAHE (Contrast Limited Adaptive Histogram Equalization): the crop is
/// split into a grid of tiles, each tile builds a clipped luminance histogram
/// (the clip caps how much any single brightness can be amplified, which is what
/// stops noise from blowing up in dark areas), and the resulting tone curves are
/// bilinearly blended between tiles so there are no visible tile seams. Only the
/// luminance is remapped; the original hue is preserved by scaling R/G/B by the
/// luminance ratio. The net effect: a face shot in dim, uneven light comes out
/// looking evenly lit to the mesh model, so eyelid/lip motion produces a real
/// signal instead of being lost in the noise floor.
///
/// `n_tiles` is the grid size per axis (8 is a good default for ~128–192px
/// crops). `clip_factor` caps amplification: 1.0 ≈ none, 2–4 is typical; higher
/// = punchier contrast but more noise.
pub fn clahe(img: &DynamicImage, n_tiles: usize, clip_factor: f32) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    if w == 0 || h == 0 {
        return img.clone();
    }
    let nx = n_tiles.max(1);
    let ny = n_tiles.max(1);
    // ceil division so the last (partial) tile still covers the edge pixels
    let tw = (w + nx - 1) / nx;
    let th = (h + ny - 1) / ny;

    // --- luminance buffer (Rec.601) ---
    let mut luma = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let p = rgba.get_pixel(x as u32, y as u32);
            let l = 0.299 * p[0] as f32 + 0.587 * p[1] as f32 + 0.114 * p[2] as f32;
            luma[y * w + x] = l.round().clamp(0.0, 255.0) as u8;
        }
    }

    // --- build one clipped-CDF tone curve (LUT) per tile ---
    const N_BINS: usize = 256;
    let mut luts = vec![[0u8; N_BINS]; nx * ny];
    for ty in 0..ny {
        for tx in 0..nx {
            let x0 = tx * tw;
            let x1 = ((tx + 1) * tw).min(w);
            let y0 = ty * th;
            let y1 = ((ty + 1) * th).min(h);

            let mut hist = [0u32; N_BINS];
            let mut npx = 0u32;
            for yy in y0..y1 {
                for xx in x0..x1 {
                    hist[luma[yy * w + xx] as usize] += 1;
                    npx += 1;
                }
            }

            let lut = &mut luts[ty * nx + tx];
            if npx == 0 {
                for (i, v) in lut.iter_mut().enumerate() {
                    *v = i as u8;
                }
                continue;
            }

            // Clip histogram peaks and redistribute the clipped mass uniformly.
            let clip_limit = ((clip_factor * npx as f32 / N_BINS as f32).max(1.0)) as u32;
            let mut excess = 0u32;
            for c in hist.iter_mut() {
                if *c > clip_limit {
                    excess += *c - clip_limit;
                    *c = clip_limit;
                }
            }
            let per_bin = excess / N_BINS as u32;
            let residual = (excess % N_BINS as u32) as usize;
            for c in hist.iter_mut() {
                *c += per_bin;
            }
            for c in hist.iter_mut().take(residual) {
                *c += 1;
            }

            // Cumulative distribution -> tone curve in 0..255.
            let mut cdf = 0u32;
            for i in 0..N_BINS {
                cdf += hist[i];
                lut[i] = ((cdf as f32 / npx as f32) * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    // --- apply with bilinear blending between the 4 nearest tile curves ---
    let mut out: RgbaImage = rgba.clone();
    for y in 0..h {
        let fy = (y as f32 + 0.5) / th as f32 - 0.5;
        let ty0 = fy.floor().clamp(0.0, (ny - 1) as f32) as usize;
        let ty1 = (ty0 + 1).min(ny - 1);
        let wy = (fy - ty0 as f32).clamp(0.0, 1.0);
        for x in 0..w {
            let fx = (x as f32 + 0.5) / tw as f32 - 0.5;
            let tx0 = fx.floor().clamp(0.0, (nx - 1) as f32) as usize;
            let tx1 = (tx0 + 1).min(nx - 1);
            let wx = (fx - tx0 as f32).clamp(0.0, 1.0);

            let l = luma[y * w + x] as usize;
            let m00 = luts[ty0 * nx + tx0][l] as f32;
            let m01 = luts[ty0 * nx + tx1][l] as f32;
            let m10 = luts[ty1 * nx + tx0][l] as f32;
            let m11 = luts[ty1 * nx + tx1][l] as f32;
            let top = m00 * (1.0 - wx) + m01 * wx;
            let bot = m10 * (1.0 - wx) + m11 * wx;
            let new_l = top * (1.0 - wy) + bot * wy;

            let old_l = luma[y * w + x] as f32;
            let src = rgba.get_pixel(x as u32, y as u32);
            let new_px = if old_l > 1.0 {
                // Preserve hue: scale each channel by the luminance ratio.
                let ratio = new_l / old_l;
                Rgba([
                    (src[0] as f32 * ratio).round().clamp(0.0, 255.0) as u8,
                    (src[1] as f32 * ratio).round().clamp(0.0, 255.0) as u8,
                    (src[2] as f32 * ratio).round().clamp(0.0, 255.0) as u8,
                    src[3],
                ])
            } else {
                // Near-black pixel carries no usable color; emit neutral gray.
                let v = new_l.round().clamp(0.0, 255.0) as u8;
                Rgba([v, v, v, src[3]])
            };
            out.put_pixel(x as u32, y as u32, new_px);
        }
    }

    DynamicImage::ImageRgba8(out)
}
