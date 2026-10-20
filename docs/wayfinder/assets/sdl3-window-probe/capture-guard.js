// THROWAWAY read-only KWin observer; config is injected by capture-compare.mjs.
var cfg = __CAPTURE_CONFIG__;
function emit(kind, state) { print(cfg.marker + " " + JSON.stringify({kind: kind, state: state})); }
function rect(r) {
    if (!r || ![r.x, r.y, r.width, r.height].every(Number.isFinite) ||
        r.width <= 0 || r.height <= 0) return null;
    return [r.x, r.y, r.width, r.height];
}
function state() {
    var w = workspace.activeWindow;
    if (!w || w.pid !== cfg.pid || w.caption !== cfg.title) return {match: false};
    var g = w.frameGeometry, c = rect(w.clientGeometry), out = w.output;
    if (!g || !c || !out) return {match: false};
    return {match: true, active: w.active, id: String(w.internalId), pid: w.pid,
        output: out.name, scale: out.devicePixelRatio, geometry: [g.x, g.y, g.width, g.height],
        clientGeometry: c};
}
if (cfg.phase === "output") {
    emit("output", {output: workspace.activeScreen.name, scale: workspace.activeScreen.devicePixelRatio});
} else {
    if (cfg.phase === "watch") {
        var w = workspace.activeWindow;
        function canConnect(s) { return s && typeof s.connect === "function"; }
        if (!state().match || !canConnect(workspace.windowActivated) ||
            !canConnect(w.frameGeometryChanged) || !canConnect(w.captionChanged) ||
            !canConnect(w.outputChanged)) {
            emit("error", {reason: "Required identity/change signals unavailable"});
        } else {
            workspace.windowActivated.connect(function () { emit("changed", {what: "activation"}); });
            w.frameGeometryChanged.connect(function () { emit("changed", {what: "geometry"}); });
            w.captionChanged.connect(function () { emit("changed", {what: "caption"}); });
            w.outputChanged.connect(function () { emit("changed", {what: "output"}); });
            if (canConnect(w.clientGeometryChanged)) w.clientGeometryChanged.connect(function () { emit("changed", {what: "clientGeometry"}); });
            var s = state(); s.armed = true; emit("watch", s);
        }
    } else { emit(cfg.phase, state()); }
}
