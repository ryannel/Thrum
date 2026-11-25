//! Shelf packing and isolated, linear-light mip generation for each frame.

use image::RgbaImage;
use thrum_asset::layout::SheetMeta;

use super::{Description, Frame, GUTTER, PreparedSheet, SHEET_MAX_MIP, Sampling, invalid};
use crate::{CookError, texture};

pub(super) fn pack(
    description: &Description,
    mut frames: Vec<Frame>,
    images: &[RgbaImage],
) -> Result<PreparedSheet, CookError> {
    let page_size = description.page_size;
    // A compressed 4×4 block at the smallest mip must belong to just one frame.
    let alignment = if matches!(description.kind, Sampling::Smooth) {
        GUTTER * 4
    } else {
        GUTTER
    };
    let (mut x, mut y, mut shelf, mut page) = (0, 0, 0, 0);
    let (mut width, mut height) = (0, 0);
    for frame in &mut frames {
        let [w, h] = frame.rect.size.map(|side| padded(side, alignment));
        if w.max(h) > page_size {
            return Err(invalid(format!(
                "{}: frame is larger than a sheet page",
                frame.name
            )));
        }
        if x + w > page_size {
            (x, y, shelf) = (0, y + shelf, 0);
        }
        if y + h > page_size {
            (x, y, shelf) = (0, 0, 0);
            page += 1;
        }
        frame.rect.page = page;
        frame.rect.origin = [x + GUTTER, y + GUTTER];
        x += w;
        shelf = shelf.max(h);
        width = width.max(x);
        height = height.max(y + shelf);
    }
    let pages = page + 1;
    let mip_len = |mip| u64::from(width >> mip) * u64::from(height >> mip) * u64::from(pages) * 4;
    if mip_len(0) > 512 * 1024 * 1024 {
        return Err(invalid("sheet pages exceed the cooker size limit"));
    }
    let mut sheet = PreparedSheet {
        records: Vec::with_capacity(frames.len()),
        names: Vec::new(),
        width,
        height,
        mips: std::array::from_fn(|mip| {
            vec![0; usize::try_from(mip_len(mip)).expect("bounded mip size")]
        }),
        meta: SheetMeta {
            pages,
            frames: index(frames.len())?,
            density: description.density,
            kind: u32::from(matches!(description.kind, Sampling::Smooth)),
        },
    };
    for mut frame in frames {
        write_frame(&mut sheet, &frame, &images[frame.image], alignment);
        frame.rect.name_offset = index(sheet.names.len())?;
        frame.rect.name_len = index(frame.name.len())?;
        sheet.names.extend_from_slice(frame.name.as_bytes());
        sheet.records.push(frame.rect);
    }
    Ok(sheet)
}

fn index(value: usize) -> Result<u32, CookError> {
    u32::try_from(value).map_err(|_| invalid("sheet frame table is too large"))
}

fn padded(size: u32, alignment: u32) -> u32 {
    (size + 2 * GUTTER).next_multiple_of(alignment)
}

fn write_frame(sheet: &mut PreparedSheet, frame: &Frame, image: &RgbaImage, alignment: u32) {
    let [width, height] = frame.rect.size.map(|side| padded(side, alignment));
    let [ox, oy] = frame.rect.origin.map(|v| v - GUTTER);
    let [w, h] = frame.rect.size;
    let mut block = vec![[0.0; 4]; (width * height) as usize];
    let source = [
        frame.cell[0] + frame.rect.trim[0],
        frame.cell[1] + frame.rect.trim[1],
    ];
    for y in 0..h {
        for x in 0..w {
            let rgba = image.get_pixel(source[0] + x, source[1] + y).0;
            let alpha = f32::from(rgba[3]) / 255.0;
            block[((y + GUTTER) * width + x + GUTTER) as usize] = [
                texture::srgb_to_linear(rgba[0]) * alpha,
                texture::srgb_to_linear(rgba[1]) * alpha,
                texture::srgb_to_linear(rgba[2]) * alpha,
                alpha,
            ];
        }
    }
    for (mip, bytes) in sheet.mips.iter_mut().enumerate() {
        let [bw, bh] = [width >> mip, height >> mip];
        if frame.rect.tile != 0 {
            extend_tile_edges(&mut block, [bw, bh], frame.rect.size, mip);
        }
        let pw = (sheet.width >> mip) as usize;
        let ph = (sheet.height >> mip) as usize;
        let page_offset = pw * ph * frame.rect.page as usize;
        for y in 0..bh as usize {
            let offset = page_offset + ((oy as usize >> mip) + y) * pw + (ox as usize >> mip);
            let out = &mut bytes[4 * offset..4 * (offset + bw as usize)];
            for (rgba, code) in block[y * bw as usize..][..bw as usize]
                .iter()
                .zip(out.as_chunks_mut::<4>().0)
            {
                let [r, g, b, a] = *rgba;
                code.copy_from_slice(&[
                    texture::linear_to_srgb(r),
                    texture::linear_to_srgb(g),
                    texture::linear_to_srgb(b),
                    texture::to_u8(a),
                ]);
            }
        }
        if mip < SHEET_MAX_MIP as usize {
            box_mip(&mut block, bw as usize, bh as usize);
        }
    }
}

fn extend_tile_edges(
    pixels: &mut [[f32; 4]],
    [width, height]: [u32; 2],
    logical: [u32; 2],
    mip: usize,
) {
    let margin = GUTTER >> mip;
    let [right, bottom] = logical.map(|side| margin + side.div_ceil(1 << mip) - 1);
    for y in 0..height {
        for x in 0..width {
            let source = (y.clamp(margin, bottom) * width + x.clamp(margin, right)) as usize;
            pixels[(y * width + x) as usize] = pixels[source];
        }
    }
}

fn box_mip(pixels: &mut Vec<[f32; 4]>, width: usize, height: usize) {
    let reduced = width / 2;
    for y in 0..height / 2 {
        for x in 0..reduced {
            let top = 2 * (y * width + x);
            let bottom = top + width;
            pixels[y * reduced + x] = std::array::from_fn(|channel| {
                (pixels[top][channel]
                    + pixels[top + 1][channel]
                    + pixels[bottom][channel]
                    + pixels[bottom + 1][channel])
                    * 0.25
            });
        }
    }
    pixels.truncate(reduced * height / 2);
}
