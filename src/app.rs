//! `src/main.c` and `src/api/api.c`: process startup, the Lua state, and the restart loop.

use std::ffi::{c_int, CStr};
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::process::ExitCode;

use mlua::ffi::{
    luaL_loadstring, luaL_newstate, luaL_openlibs, luaL_requiref, lua_State, lua_close,
    lua_createtable, lua_pcall, lua_pushboolean, lua_pushlstring, lua_pushstring, lua_rawseti,
    lua_setglobal, lua_toboolean,
};
use sdl3_sys::everything as sdl;

const VERSION: &CStr = unsafe {
    CStr::from_bytes_with_nul_unchecked(concat!(env!("LITE_PROJECT_VERSION_STR"), "\0").as_bytes())
};
const ARCH_TUPLE: &CStr = unsafe {
    CStr::from_bytes_with_nul_unchecked(concat!(env!("LITE_ARCH_TUPLE"), "\0").as_bytes())
};

/// The Lua chunk that boots the editor, verbatim from `main.c` for Linux (`HOME`, `/`).
/// It returns `core.restart_request` so the caller knows whether to start over.
const INIT_LITE_CODE: &CStr = c"local core
local os_exit = os.exit
os.exit = function(code, close)
  os_exit(code, close == nil and true or close)
end
xpcall(function()
  local match = require('utf8extra').match
  HOME = os.getenv('HOME')
  local exedir = match(EXEFILE, '^(.*)/[^/]+$')
  local prefix = os.getenv('LITE_PREFIX') or match(exedir, '^(.*)/bin$')
  dofile((MACOS_RESOURCES or (prefix and prefix .. '/share/lite-xl' or exedir .. '/data')) .. '/core/start.lua')
  core = require(os.getenv('LITE_XL_RUNTIME') or 'core')
  core.init()
  core.run()
end, function(err)
  local error_path = 'error.txt'
  io.stdout:write('Error: '..tostring(err)..'\\n')
  io.stdout:write(debug.traceback(nil, 2)..'\\n')
  if core and core.on_error then
    error_path = USERDIR .. PATHSEP .. error_path
    pcall(core.on_error, err)
  else
    local fp = io.open(error_path, 'wb')
    fp:write('Error: ' .. tostring(err) .. '\\n')
    fp:write(debug.traceback(nil, 2)..'\\n')
    fp:close()
    error_path = system.absolute_path(error_path)
  end
  system.show_fatal_error('Lite XL internal error',
    'An internal error occurred in a critical part of the application.\\n\\n'..
    'Error: '..tostring(err)..'\\n\\n'..
    'Details can be found in \\\"'..error_path..'\\\"')
  os.exit(1)
end)
return core and core.restart_request
";

/// `api_load_libs`: also the symbol native plugins resolve through the `XL` table.
///
/// # Safety
/// `l` is a live Lua state.
#[no_mangle]
pub unsafe extern "C-unwind" fn api_load_libs(l: *mut lua_State) {
    type Open = unsafe extern "C-unwind" fn(*mut lua_State) -> c_int;
    let libs: [(&CStr, Open); 7] = [
        (c"system", crate::luaopen_system),
        (c"renderer", crate::renderer_api::luaopen_renderer),
        (c"renwindow", crate::renwindow_api::luaopen_renwindow),
        (c"regex", crate::luaopen_regex),
        (c"process", crate::luaopen_process),
        (c"dirmonitor", crate::luaopen_dirmonitor),
        (c"utf8extra", crate::luaopen_utf8extra),
    ];
    for (name, open) in libs {
        luaL_requiref(l, name.as_ptr(), open, 1);
    }
}

fn exe_filename() -> Option<Vec<u8>> {
    // /proc/self/exe, like the C readlink.
    std::env::current_exe()
        .ok()
        .map(|p| p.into_os_string().into_vec())
        .filter(|p| !p.is_empty())
}

pub fn run() -> ExitCode {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);

        sdl::SDL_SetAppMetadata(
            c"Lite XL".as_ptr(),
            VERSION.as_ptr(),
            c"com.lite_xl.LiteXL".as_ptr(),
        );
        if !sdl::SDL_Init(sdl::SDL_INIT_EVENTS) {
            eprint!(
                "Error initializing sdl: {}",
                CStr::from_ptr(sdl::SDL_GetError()).to_string_lossy()
            );
            std::process::exit(1);
        }
        sdl::SDL_SetEventEnabled(sdl::SDL_EVENT_DROP_FILE.into(), true);
        extern "C" fn quit() {
            unsafe { sdl::SDL_Quit() }
        }
        libc::atexit(quit);

        crate::custom_events::init_custom_events();

        let args: Vec<_> = std::env::args_os().collect();
        let exe = exe_filename()
            .or_else(|| args.first().map(|a| a.as_bytes().to_vec()))
            .unwrap_or_default();

        let mut has_restarted = false;
        loop {
            let l = luaL_newstate();
            luaL_openlibs(l);
            api_load_libs(l);

            lua_createtable(l, args.len() as c_int, 0);
            for (i, arg) in args.iter().enumerate() {
                lua_pushlstring(l, arg.as_bytes().as_ptr() as *const _, arg.len());
                lua_rawseti(l, -2, i as mlua::ffi::lua_Integer + 1);
            }
            lua_setglobal(l, c"ARGS".as_ptr());

            lua_pushstring(l, sdl::SDL_GetPlatform());
            lua_setglobal(l, c"PLATFORM".as_ptr());
            lua_pushstring(l, ARCH_TUPLE.as_ptr());
            lua_setglobal(l, c"ARCH".as_ptr());
            lua_pushboolean(l, has_restarted as c_int);
            lua_setglobal(l, c"RESTARTED".as_ptr());
            lua_pushlstring(l, exe.as_ptr() as *const _, exe.len());
            lua_setglobal(l, c"EXEFILE".as_ptr());

            sdl::SDL_SetEventEnabled(sdl::SDL_EVENT_TEXT_INPUT.into(), true);
            sdl::SDL_SetEventEnabled(sdl::SDL_EVENT_TEXT_EDITING.into(), true);

            if luaL_loadstring(l, INIT_LITE_CODE.as_ptr()) != 0 {
                eprintln!("internal error when starting the application");
                std::process::exit(1);
            }
            // The chunk handles its own errors (xpcall); the status is not needed.
            lua_pcall(l, 0, 1, 0);
            let restart = lua_toboolean(l, -1) != 0;
            lua_close(l);
            if restart {
                crate::rencache::rencache_invalidate();
                has_restarted = true;
                continue;
            }
            break;
        }

        // The event loop is over, so the custom events can go.
        crate::custom_events::free_custom_events();
    }
    ExitCode::SUCCESS
}
