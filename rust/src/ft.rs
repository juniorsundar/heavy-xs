//! FreeType operations missing from freetype-rs: custom LCD weights, bitmap metrics-only
//! loading, and SDL streams (needed for Unicode filenames on Windows).
use core::ffi::{c_int, c_ulong, c_void};
use freetype::ffi as ft;
use sdl3_sys::everything as sdl;

pub const FT_LOAD_BITMAP_METRICS_ONLY: ft::FT_Int32 = 1 << 22;

extern "C" {
    pub fn FT_Library_SetLcdFilterWeights(
        library: ft::FT_Library,
        weights: *mut ft::FT_Byte,
    ) -> ft::FT_Error;
}

// freetype-sys declares callbacks as safe extern functions; only FreeType calls them.
extern "C" fn font_file_read(
    stream: ft::FT_Stream,
    offset: c_ulong,
    buffer: *mut ft::FT_Byte,
    count: c_ulong,
) -> c_ulong {
    unsafe {
        let file = (*stream).descriptor as *mut sdl::SDL_IOStream;
        sdl::SDL_SeekIO(file, offset as c_int as sdl::Sint64, sdl::SDL_IO_SEEK_SET);
        if count == 0 {
            return 0;
        }
        sdl::SDL_ReadIO(file, buffer as *mut c_void, count as usize) as c_ulong
    }
}

extern "C" fn font_file_close(stream: ft::FT_Stream) {
    unsafe {
        if !stream.is_null() {
            let file = (*stream).descriptor as *mut sdl::SDL_IOStream;
            if !file.is_null() {
                sdl::SDL_CloseIO(file);
            }
            drop(Box::from_raw(stream));
        }
    }
}

/// Transfer ownership of `file` to a FreeType stream, closed by FT_Open_Face on failure
/// or by the face destructor on success.
pub unsafe fn font_stream_new(file: *mut sdl::SDL_IOStream) -> ft::FT_Stream {
    Box::into_raw(Box::new(ft::FT_StreamRec {
        base: std::ptr::null_mut(),
        size: sdl::SDL_GetIOSize(file) as c_ulong,
        pos: 0,
        descriptor: file as *mut c_void,
        pathname: std::ptr::null_mut(),
        read: font_file_read,
        close: font_file_close,
        memory: std::ptr::null_mut(),
        cursor: std::ptr::null_mut(),
        limit: std::ptr::null_mut(),
    }))
}
