use crate::utils::{
    encoding::base64_encode_standard,
    nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
    strings::extract_utf16_string,
};
use common::windows::{ColorFlags, Console, CursorSize, FontFamily, FontWeight};
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

/// Determine if extra Console Properties data exists in `Shortcut` data
pub(crate) fn has_console(data: &[u8]) -> (bool, Vec<Console>) {
    let result = parse_console(data);
    match result {
        Ok((_, console)) => (true, console),
        Err(_err) => (false, Vec::new()),
    }
}

/// Parse `Shortcut` Console info
fn parse_console(data: &[u8]) -> nom::IResult<&[u8], Vec<Console>> {
    let sig = [2, 0, 0, 160];
    let (_, sig_start) = take_until(sig.as_slice())(data)?;

    let adjust_start = 4;
    let (console_data, _) = take(sig_start.len() - adjust_start)(data)?;
    let (input, _size_data) = take(size_of::<u32>())(console_data)?;
    let (input, _sig_data) = take(size_of::<u32>())(input)?;

    let (input, color_flags) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, popup_fill_attributes) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, screen_width_buffer_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, screen_height_buffer_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, window_width) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, window_height) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, window_x_coordinate) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, window_y_coordinate) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (input, _reserved) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _reserved) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (input, font_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, font_family) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, font_weight) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let string_size: u8 = 64;
    let (input, string_data) = take(string_size)(input)?;

    let (input, cursor_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, full_screen) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, insert_mode) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, automatic_position) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, history_buffer_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, number_history_buffers) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, duplicates_allowed_history) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, color_table) = take(string_size)(input)?;

    let console = Console {
        color_flags: get_color(&color_flags),
        popup_fill_attributes: get_color(&popup_fill_attributes),
        screen_width_buffer_size,
        screen_height_buffer_size,
        window_width,
        window_height,
        window_x_coordinate,
        window_y_coordinate,
        font_size,
        font_family: get_family(&font_family),
        font_weight: get_weight(&font_weight),
        face_name: extract_utf16_string(string_data),
        cursor_size: get_cursor(&cursor_size),
        full_screen,
        insert_mode,
        automatic_position,
        history_buffer_size,
        number_history_buffers,
        duplicates_allowed_history,
        color_table: base64_encode_standard(color_table),
    };

    Ok((input, vec![console]))
}

/// Get Console Color Flags
fn get_color(color: &u16) -> Vec<ColorFlags> {
    let fore_blue = 0x1;
    let fore_green = 0x2;
    let fore_red = 0x4;
    let fore_intense = 0x8;

    let back_blue = 0x10;
    let back_green = 0x20;
    let back_red = 0x40;
    let back_intense = 0x80;

    let mut colors = Vec::new();
    if (color & fore_blue) == fore_blue {
        colors.push(ColorFlags::ForegroundBlue);
    }
    if (color & fore_green) == fore_green {
        colors.push(ColorFlags::ForegroundGreen);
    }
    if (color & fore_red) == fore_red {
        colors.push(ColorFlags::ForegroundRed);
    }
    if (color & fore_intense) == fore_intense {
        colors.push(ColorFlags::ForegroundIntensity);
    }

    if (color & back_blue) == back_blue {
        colors.push(ColorFlags::BackgroundBlue);
    }
    if (color & back_green) == back_green {
        colors.push(ColorFlags::BackgroundGreen);
    }
    if (color & back_red) == back_red {
        colors.push(ColorFlags::BackgroundRed);
    }
    if (color & back_intense) == back_intense {
        colors.push(ColorFlags::BackgroundIntensity);
    }

    colors
}

/// Get Font Family
fn get_family(font: &u32) -> FontFamily {
    // Font Family is last 28 bits. First 4 bits may be Font Pitch? (https://github.com/Velocidex/velociraptor/blob/master/artifacts/definitions/Windows/Forensics/Lnk.yaml#L721)
    let start_bit = 3;
    let bits = 27;
    let bit_value = ((1 << bits) - 1) << start_bit;

    let font_value = font & bit_value;
    match font_value {
        0x0 => FontFamily::DontCare,
        0x10 => FontFamily::Roman,
        0x20 => FontFamily::Swiss,
        0x30 => FontFamily::Modern,
        0x40 => FontFamily::Script,
        0x50 => FontFamily::Decorative,
        _ => FontFamily::Unknown,
    }
}

/// Get Font Weight
fn get_weight(font: &u32) -> FontWeight {
    let regular = 700;

    if font < &regular {
        FontWeight::Regular
    } else {
        FontWeight::Bold
    }
}

/// Get Cursor Size
fn get_cursor(cursor: &u32) -> CursorSize {
    let small = 25;
    let normal = 50;
    let large = 100;

    if cursor <= &small {
        CursorSize::Small
    } else if cursor > &small && cursor <= &normal {
        CursorSize::Normal
    } else if cursor > &normal && cursor <= &large {
        CursorSize::Large
    } else {
        CursorSize::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::{get_cursor, get_weight, has_console};
    use crate::{
        artifacts::os::windows::shortcuts::extras::console::{
            get_color, get_family, parse_console, ColorFlags, CursorSize, FontFamily, FontWeight,
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_has_console() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/windows/lnk/win2012/Windows PowerShell (x86).lnk");
        let result = read_file(&test_location.display().to_string()).unwrap();

        let (has_console, console) = has_console(&result);
        assert!(has_console);

        assert_eq!(console[0].font_family, FontFamily::Modern);
        assert_eq!(console[0].font_size, 12);
        assert_eq!(console[0].face_name, "Lucida Console");
    }

    #[test]
    fn test_parse_console() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/windows/lnk/win2012/Windows PowerShell (x86).lnk");
        let result = read_file(&test_location.display().to_string()).unwrap();

        let (_, console) = parse_console(&result).unwrap();

        assert_eq!(console[0].font_family, FontFamily::Modern);
        assert_eq!(console[0].font_size, 12);
        assert_eq!(console[0].face_name, "Lucida Console");
    }

    #[test]
    fn test_get_cursor() {
        let test = 99;
        let result = get_cursor(&test);
        assert_eq!(result, CursorSize::Large);
    }

    #[test]
    fn test_get_weight() {
        let test = 800;
        let result = get_weight(&test);
        assert_eq!(result, FontWeight::Bold);
    }

    #[test]
    fn test_get_family() {
        let test = 0x36;
        let result = get_family(&test);
        assert_eq!(result, FontFamily::Modern);
    }

    #[test]
    fn test_get_color() {
        let test = 12;
        let result = get_color(&test);
        assert_eq!(
            result,
            [ColorFlags::ForegroundRed, ColorFlags::ForegroundIntensity]
        );
    }
}
