//! Wayfinder additional-font-format reference probe.
//!
//! One `FT_OpenFace` face-0 metadata case per process:
//! `font-format-metadata <id> <path> <size> <hex_codepoint>`.
//! Prints one TSV header and one row, reporting FreeType failures as fields.
//! Inspects metrics and any already-loaded small mono bitmap; it does not render.
use freetype::face::LoadFlag;
use freetype::ffi as ft;
use freetype::{Face, Library};

/// Same value as `src/ft.rs` and the `src/renderer.rs` metric path.
const FT_LOAD_BITMAP_METRICS_ONLY: ft::FT_Int32 = 1 << 22;

const HEADER: &str = "id\tface0_family\tscalable\tfixed_sizes_ppem\tface_height_units\t\
face_open_status\tset_size_status\tcp\tglyph_id\tglyph_load_status\tglyph_format\t\
bitmap_pixel_mode\tbitmap_width\tbitmap_rows\toutline_points\tbitmap_buffer_present\tmono_bitmap_hex";

fn pixel_mode_name(mode: u32) -> &'static str {
    match mode {
        ft::FT_PIXEL_MODE_NONE => "none",
        ft::FT_PIXEL_MODE_MONO => "mono",
        ft::FT_PIXEL_MODE_GRAY => "gray",
        ft::FT_PIXEL_MODE_GRAY2 => "gray2",
        ft::FT_PIXEL_MODE_GRAY4 => "gray4",
        ft::FT_PIXEL_MODE_LCD => "lcd",
        ft::FT_PIXEL_MODE_LCD_V => "lcd_v",
        ft::FT_PIXEL_MODE_BGRA => "bgra",
        _ => "other",
    }
}

/// Glyph slot FourCC (`outl`, `bits`, ...) or hex when unprintable.
fn format_tag(format: u32) -> String {
    let bytes = format.to_be_bytes();
    if bytes.iter().all(u8::is_ascii_graphic) {
        String::from_utf8_lossy(&bytes).into_owned()
    } else {
        format!("{format:#010x}")
    }
}

/// Available strike sizes in ppem (`FT_Bitmap_Size::y_ppem / 64`), or `-`.
fn fixed_sizes_ppem(face: &Face) -> String {
    let count = face.raw().num_fixed_sizes as isize;
    if count <= 0 {
        return "-".into();
    }
    let sizes = face.raw().available_sizes;
    (0..count)
        .map(|i| {
            let y = unsafe { (*sizes.add(i as usize)).y_ppem } as f64 / 64.0;
            if y.fract() == 0.0 {
                format!("{}", y as i64)
            } else {
                format!("{y}")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn row(id: &str, cp: u32, open_status: &str, face: Option<(&Face, u32)>) -> String {
    let cp_field = format!("U+{cp:04X}");
    let mut fields = vec![
        id.to_string(),
        "-".into(),
        "-".into(),
        "-".into(),
        "-".into(),
        open_status.to_string(),
    ];
    match face {
        None => {
            fields.push("-".into());
            fields.push(cp_field);
            fields.push("0".into());
            fields.extend(["-"; 8].map(|_| "-".to_string()));
        }
        Some((face, size)) => {
            fields[1] = face
                .family_name()
                .unwrap_or_else(|| "-".into())
                .replace(['\t', '\r', '\n'], " ");
            fields[2] = face.is_scalable().to_string();
            fields[3] = fixed_sizes_ppem(face);
            fields[4] = face.height().to_string();
            let glyph_id = face.get_char_index(cp as usize).unwrap_or(0);
            let mut load: [String; 9] = std::array::from_fn(|_| "-".to_string());
            match face.set_pixel_sizes(0, size) {
                Err(e) => load[0] = format!("err({e:?})"),
                Ok(()) => {
                    let flags = LoadFlag::from_bits_retain(FT_LOAD_BITMAP_METRICS_ONLY)
                        | LoadFlag::NO_HINTING;
                    load[0] = "ok".into();
                    match face.load_char(cp as usize, flags) {
                        Err(e) => load[1] = format!("err({e:?})"),
                        Ok(()) => {
                            let slot = face.glyph().raw();
                            let bitmap = face.glyph().bitmap();
                            let mono_hex = if slot.bitmap.pixel_mode as u32
                                == ft::FT_PIXEL_MODE_MONO
                                && !slot.bitmap.buffer.is_null()
                                && bitmap.buffer().len() <= 4096
                            {
                                bitmap.buffer().iter().map(|b| format!("{b:02x}")).collect()
                            } else {
                                "-".into()
                            };
                            load = [
                                "ok".into(),
                                "ok".into(),
                                format_tag(slot.format as u32),
                                pixel_mode_name(slot.bitmap.pixel_mode as u32).into(),
                                slot.bitmap.width.to_string(),
                                slot.bitmap.rows.to_string(),
                                slot.outline.n_points.to_string(),
                                (!slot.bitmap.buffer.is_null()).to_string(),
                                mono_hex,
                            ];
                        }
                    }
                }
            }
            fields.push(load[0].clone());
            fields.push(cp_field);
            fields.push(glyph_id.to_string());
            fields.extend(load.into_iter().skip(1));
        }
    }
    fields.join("\t")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 {
        eprintln!(
            "usage: {} <id> <path> <size> <hex_codepoint>",
            args.first()
                .map(String::as_str)
                .unwrap_or("font-format-metadata")
        );
        std::process::exit(2);
    }
    let (id, path) = (&args[1], &args[2]);
    let size = args[3]
        .parse::<u32>()
        .ok()
        .filter(|n| (1..=256).contains(n));
    let cp = u32::from_str_radix(args[4].trim_start_matches("0x"), 16)
        .ok()
        .filter(|n| char::from_u32(*n).is_some());
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        || size.is_none()
        || cp.is_none()
    {
        eprintln!("invalid id, size (1..256), or Unicode scalar");
        std::process::exit(2);
    }
    let (size, cp) = (size.unwrap(), cp.unwrap());

    println!("{HEADER}");
    let line = match Library::init() {
        Err(e) => row(id, cp, &format!("err({e:?})"), None),
        Ok(library) => match library.new_face(path, 0) {
            Err(e) => row(id, cp, &format!("err({e:?})"), None),
            Ok(face) => row(id, cp, "ok", Some((&face, size))),
        },
    };
    println!("{line}");
}
