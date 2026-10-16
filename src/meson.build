lite_sources = [
    'api/api.c',
    'api/renwindow.c',
    'arena_allocator.c',
    'main.c',
]

#===============================================================================
# Dependencies
#===============================================================================
libm = cc.find_library('m', required : false)
libdl = cc.find_library('dl', required : false)

default_fallback_options = ['warning_level=0', 'werror=false']

# Lua has no official .pc file
# so distros come up with their own names
lua_names = [
    'lua5.4', # Debian
    'lua-5.4', # FreeBSD
    'lua',    # Fedora
]

if get_option('use_system_lua')
    foreach lua : lua_names
        last_lua = (lua == lua_names[-1] or get_option('wrap_mode') == 'forcefallback')
        lua_dep = dependency(lua, required : false,
        )
        if lua_dep.found()
            break
        endif

        if last_lua
            # If we could not find lua on the system and fallbacks are disabled
            # try the compiler as a last ditch effort, since Lua has no official
            # pkg-config support.
            lua_dep = cc.find_library('lua', required : true)
        endif
    endforeach
else
    lua_dep = dependency('', fallback: ['lua', 'lua_dep'], required : true,
        default_options: default_fallback_options + ['default_library=static', 'line_editing=disabled', 'interpreter=false']
    )
endif

pcre2_dep = dependency('libpcre2-8', fallback: ['pcre2', 'libpcre2_8'],
    default_options: default_fallback_options + ['default_library=static', 'grep=false', 'test=false']
)

sdl_dep = dependency('sdl3', static: true)

# freetype-rs supplies the bindings, but W7 retains Meson's pinned FreeType for pixel parity.
# Its bundled 2.13.2 changes hinted glyphs; move build ownership separately in W8.
# Cargo probes Meson's uninstalled .pc first, so a fallback build needs no system FreeType.
freetype_dep = dependency('freetype2', fallback: ['freetype2', 'freetype_dep'],
    default_options: default_fallback_options + ['default_library=static', 'zlib=disabled', 'bzip2=disabled', 'png=disabled', 'harfbuzz=disabled', 'brotli=disabled']
)
lite_deps = [lua_dep, sdl_dep, freetype_dep, pcre2_dep, libm, libdl]

# Rust core (docs/rust-port/PLAN.md). cargo owns incremental builds, so always run it.
rust_profile = get_option('buildtype') == 'release' ? 'release' : 'debug'
# LITE_USE_SDL_RENDERER (macOS, or -Drenderer=true) swaps the window's surface handling in
# rust/src/renwindow.rs; the Rust core needs to be told with a cargo feature.
# `module`: this build supplies Lua, PCRE2, FreeType and SDL3 itself (see rust/Cargo.toml).
rust_features = 'module'
if get_option('core_features') != ''
    rust_features += ',' + get_option('core_features')
endif
if get_option('renderer') or host_machine.system() == 'darwin'
    rust_features += ',sdl_renderer'
endif
rust_target_dir = meson.project_build_root() / 'rust-target'
rust_lib = custom_target('lite_xl_core',
    output: 'liblite_xl_core.a',
    command: [
        find_program('sh'), '-c',
        'PKG_CONFIG_PATH="$7${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}" cargo build --manifest-path "$1/Cargo.toml" --target-dir "$2" --profile "$3" --features "$4" --no-default-features && cp "$2/$5/liblite_xl_core.a" "$6"',
        'sh', meson.project_source_root() / 'rust', rust_target_dir,
        rust_profile == 'debug' ? 'dev' : 'release', rust_features,
        rust_profile, '@OUTPUT@', meson.project_build_root() / 'meson-uninstalled',
    ],
    console: true,
    build_by_default: true,
    build_always_stale: true,
)
lite_deps += declare_dependency(sources: rust_lib, link_args: ['-lpthread', '-ldl', '-lm'])

# dirmonitor backend. On Linux (inotify) the whole module lives in rust/src/dirmonitor.rs;
# the other platforms still use the C module with their backend.
# shortcut: the Rust module is compiled on Linux only, so a non-Linux host must not
# pick the inotify backend; port the other backends before dropping dirmonitor.c.
if get_option('dirmonitor_backend') == ''
    if cc.has_function('inotify_init', prefix : '#include<sys/inotify.h>')
        dirmonitor_backend = 'inotify'
    elif host_machine.system() == 'darwin' and cc.check_header('CoreServices/CoreServices.h')
        dirmonitor_backend = 'fsevents'
    elif cc.has_function('kqueue', prefix : '#include<sys/event.h>')
        dirmonitor_backend = 'kqueue'
    elif cc.has_function('create_inode_watcher', prefix : '#include<fcntl.h>')
        dirmonitor_backend = 'inodewatcher'
    elif dependency('libkqueue', required : false).found()
        dirmonitor_backend = 'kqueue'
    elif host_machine.system() == 'windows'
        dirmonitor_backend = 'win32'
    else
        dirmonitor_backend = 'dummy'
        warning('no suitable backend found, defaulting to dummy backend')
    endif
else
    dirmonitor_backend = get_option('dirmonitor_backend')
endif

if dirmonitor_backend != 'inotify'
    lite_sources += 'api/dirmonitor.c'
endif

if dirmonitor_backend == 'inotify'
    # Nothing to compile: rust/src/dirmonitor.rs provides luaopen_dirmonitor.
    message('dirmonitor: Rust inotify implementation')
elif dirmonitor_backend == 'fsevents'
    lite_sources += 'api' / 'dirmonitor' / 'fsevents.c'
elif dirmonitor_backend == 'kqueue'
    lite_sources += 'api' / 'dirmonitor' / 'kqueue.c'
    libkqueue_dep = dependency('libkqueue', required : false)
    if libkqueue_dep.found()
        lite_deps += libkqueue_dep
    endif
elif dirmonitor_backend == 'inodewatcher'
    add_languages('cpp')
    lite_sources += 'api' / 'dirmonitor' / 'inodewatcher.cpp'
elif dirmonitor_backend == 'win32'
    lite_sources += 'api' / 'dirmonitor' / 'win32.c'
else
    lite_sources += 'api' / 'dirmonitor' / 'dummy.c'
endif

message('dirmonitor_backend: @0@'.format(dirmonitor_backend))

lite_rc = []
if host_machine.system() == 'windows'
    windows = import('windows')
    lite_rc += windows.compile_resources('..' / 'resources' / 'icons' / 'icon.rc')
    lite_rc += windows.compile_resources('..' / 'resources' / 'windows' / 'manifest.rc')
elif host_machine.system() == 'darwin'
    add_languages('objc')
    lite_sources += 'bundle_open.m'
endif

lite_includes += include_directories('.')

executable('lite-xl',
    lite_sources + lite_rc,
    include_directories: lite_includes,
    dependencies: lite_deps,
    c_args: lite_cargs,
    objc_args: lite_cargs,
    link_args: lite_link_args,
    install_dir: lite_bindir,
    install: true,
    win_subsystem: 'windows',
)
