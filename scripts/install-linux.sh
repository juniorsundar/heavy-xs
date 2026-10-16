#!/usr/bin/env sh
# Installs a Cargo-built lite-xl on Linux; replaces `meson install` for the standalone build.
#   scripts/install-linux.sh BINARY [DESTDIR] [PREFIX]      (PREFIX default /usr/local)
# Layout matches meson.build: bin/, share/lite-xl/{core,plugins,colors,fonts,...}, desktop file,
# metainfo, icon and licenses.
set -eu
root=$(cd "$(dirname "$0")/.." && pwd)
bin=${1:?usage: install-linux.sh BINARY [DESTDIR] [PREFIX]}
dest=${2:-}
prefix=${3:-/usr/local}
root_out=$dest$prefix
data=$root_out/share/lite-xl

# Same version string the binary reports (rust/build.rs): the release profile has no git suffix.
version=$(sed -n 's/^version = "\(.*\)"/\1/p' "$root/rust/Cargo.toml" | head -1)
if [ "${LITE_RELEASE:-}" != 1 ]; then
  commit=$(git -C "$root" rev-parse HEAD 2>/dev/null | cut -c1-8 || true)
  [ -n "$commit" ] && version="$version (git-$commit)"
fi

install -Dm755 "$bin" "$root_out/bin/lite-xl"
mkdir -p "$data" "$root_out/share/doc/lite-xl"
cp -r "$root/docs/api/." "$data/"
mkdir -p "$data/core"
for f in "$root"/data/core/*; do [ "$(basename "$f")" = start.lua ] || cp -r "$f" "$data/core/"; done
for d in fonts plugins colors; do cp -r "$root/data/$d" "$data/"; done
# start.lua is a template: only @PROJECT_VERSION@ is substituted.
sed "s|@PROJECT_VERSION@|$version|" "$root/data/core/start.lua" > "$data/core/start.lua"
install -Dm644 "$root/licenses/licenses.md" "$root_out/share/doc/lite-xl/licenses.md"
install -Dm644 "$root/resources/icons/lite-xl.svg" "$root_out/share/icons/hicolor/scalable/apps/lite-xl.svg"
install -Dm644 "$root/resources/linux/com.lite_xl.LiteXL.desktop" "$root_out/share/applications/com.lite_xl.LiteXL.desktop"

# <releases> from the version tags, as meson.build builds it (v1.0x tags and pre-releases skipped).
releases=$(git -C "$root" tag -l --sort=-version:refname --format='%(refname:strip=2) %(creatordate:short)' 2>/dev/null |
  awk '$1 ~ /^v/ && $1 !~ /-/ && $1 !~ /^v1\.0/ && NF == 2 { printf "<release version=\"%s\" date=\"%s\" />\n", $1, $2 }' || true)
if [ -n "$releases" ]; then releases="<releases>
$releases
</releases>"; fi
mkdir -p "$root_out/share/metainfo"
awk -v r="$releases" '{ if ($0 ~ /@METAINFO_RELEASES@/) print r; else print }' \
  "$root/resources/linux/com.lite_xl.LiteXL.metainfo.xml.in" > "$root_out/share/metainfo/com.lite_xl.LiteXL.metainfo.xml"
