// RunWee lesson harness (QuickJS).
//
// Loads an anims_bms topic module unchanged and drives it the way app.js does:
// the lecture/intro timelines switch presets, set the mode, run each step's
// `action(cb)` (slider tweens chained through callbacks), step the physics every
// frame and call `draw(ctx, 800, 400, state, params, simState, time)`. The
// canvas is a recorder: every call becomes a command that RunWee replays into
// vector graphics. Math-overlay lines are captured through the
// core/renderer.js shim (see `renderer` below).

const V_W = 800, V_H = 400;

// ───────────────────────────── canvas recorder ─────────────────────────────

class RecGradient {
    constructor(kind, args) { this.kind = kind; this.args = args; this.stops = []; }
    addColorStop(offset, color) { this.stops.push([+offset, String(color)]); }
}

const STYLE_DEFAULTS = {
    fillStyle: '#000000', strokeStyle: '#000000', lineWidth: 1, font: '10px sans-serif',
    textAlign: 'start', textBaseline: 'alphabetic', globalAlpha: 1, lineCap: 'butt',
    lineJoin: 'miter', lineDashOffset: 0, shadowColor: 'rgba(0, 0, 0, 0)', shadowBlur: 0,
    shadowOffsetX: 0, shadowOffsetY: 0, globalCompositeOperation: 'source-over',
    miterLimit: 10, imageSmoothingEnabled: true, filter: 'none', direction: 'inherit',
    letterSpacing: '0px', fontKerning: 'auto',
};

function paintCmd(prop, v) {
    if (v instanceof RecGradient) return [prop + 'Grad', v.kind, v.args, v.stops];
    return [prop, String(v)];
}

export class RecCtx {
    constructor() {
        this.c = [];
        this._s = Object.assign({}, STYLE_DEFAULTS);
        this._dash = [];
        this._stack = [];
        this.canvas = { width: V_W, height: V_H, style: {}, getBoundingClientRect: () => ({ width: V_W, height: V_H, left: 0, top: 0 }) };
    }
    // state
    save() { this._stack.push([Object.assign({}, this._s), this._dash.slice()]); this.c.push(['save']); }
    restore() { const s = this._stack.pop(); if (s) { this._s = s[0]; this._dash = s[1]; } this.c.push(['restore']); }
    reset() { this._s = Object.assign({}, STYLE_DEFAULTS); this._dash = []; this._stack = []; this.c.push(['resetAll']); }
    // transforms
    translate(x, y) { this.c.push(['translate', +x, +y]); }
    rotate(a) { this.c.push(['rotate', +a]); }
    scale(x, y) { this.c.push(['scale', +x, +(y === undefined ? x : y)]); }
    transform(a, b, c, d, e, f) { this.c.push(['transform', +a, +b, +c, +d, +e, +f]); }
    setTransform(a, b, c, d, e, f) {
        if (typeof a === 'object' && a) { const m = a; this.c.push(['setTransform', m.a ?? 1, m.b ?? 0, m.c ?? 0, m.d ?? 1, m.e ?? 0, m.f ?? 0]); return; }
        if (a === undefined) { this.c.push(['setTransform', 1, 0, 0, 1, 0, 0]); return; }
        this.c.push(['setTransform', +a, +b, +c, +d, +e, +f]);
    }
    resetTransform() { this.c.push(['setTransform', 1, 0, 0, 1, 0, 0]); }
    getTransform() { return { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0, is2D: true, isIdentity: true }; }
    // paths
    beginPath() { this.c.push(['beginPath']); }
    closePath() { this.c.push(['closePath']); }
    moveTo(x, y) { this.c.push(['moveTo', +x, +y]); }
    lineTo(x, y) { this.c.push(['lineTo', +x, +y]); }
    quadraticCurveTo(a, b, x, y) { this.c.push(['quad', +a, +b, +x, +y]); }
    bezierCurveTo(a, b, c, d, x, y) { this.c.push(['bezier', +a, +b, +c, +d, +x, +y]); }
    arc(x, y, r, a0, a1, ccw) { this.c.push(['arc', +x, +y, Math.abs(+r), +a0, +a1, !!ccw]); }
    arcTo(x1, y1, x2, y2, r) { this.c.push(['arcTo', +x1, +y1, +x2, +y2, +r]); }
    ellipse(x, y, rx, ry, rot, a0, a1, ccw) { this.c.push(['ellipse', +x, +y, Math.abs(+rx), Math.abs(+ry), +rot, +a0, +a1, !!ccw]); }
    rect(x, y, w, h) { this.c.push(['rect', +x, +y, +w, +h]); }
    roundRect(x, y, w, h, r) {
        let radii = r === undefined ? [0] : (Array.isArray(r) ? r : [r]);
        radii = radii.map(v => (typeof v === 'object' && v) ? +(v.x ?? 0) : +v);
        this.c.push(['roundRect', +x, +y, +w, +h, radii]);
    }
    // painting
    fill(a, b) { const rule = typeof a === 'string' ? a : (typeof b === 'string' ? b : 'nonzero'); this.c.push(['fill', rule]); }
    stroke() { this.c.push(['stroke']); }
    clip() { this.c.push(['clip']); }
    fillRect(x, y, w, h) { this.c.push(['fillRect', +x, +y, +w, +h]); }
    strokeRect(x, y, w, h) { this.c.push(['strokeRect', +x, +y, +w, +h]); }
    clearRect(x, y, w, h) { this.c.push(['clearRect', +x, +y, +w, +h]); }
    fillText(t, x, y) { this.c.push(['fillText', String(t), +x, +y]); }
    strokeText(t, x, y) { this.c.push(['strokeText', String(t), +x, +y]); }
    measureText(t) {
        const px = parseFloat((/([\d.]+)px/.exec(this._s.font) || [0, 10])[1]);
        const width = __rw_measure(this._s.font, String(t));
        return {
            width, actualBoundingBoxAscent: px * 0.72, actualBoundingBoxDescent: px * 0.22,
            actualBoundingBoxLeft: 0, actualBoundingBoxRight: width,
            fontBoundingBoxAscent: px * 0.97, fontBoundingBoxDescent: px * 0.24,
        };
    }
    setLineDash(seg) { this._dash = Array.isArray(seg) ? seg.map(Number) : []; this.c.push(['dash', this._dash]); }
    getLineDash() { return this._dash.slice(); }
    createLinearGradient(x0, y0, x1, y1) { return new RecGradient('linear', [+x0, +y0, +x1, +y1]); }
    createRadialGradient(x0, y0, r0, x1, y1, r1) { return new RecGradient('radial', [+x0, +y0, +r0, +x1, +y1, +r1]); }
    createConicGradient(a, x, y) { return new RecGradient('linear', [x - 1, y, x + 1, y]); }
    createPattern() { return null; }
    drawImage() {}
    putImageData() {}
    getImageData(x, y, w, h) { return { width: w, height: h, data: new Uint8ClampedArray(Math.max(0, w * h * 4)) }; }
    createImageData(w, h) { return { width: w, height: h, data: new Uint8ClampedArray(Math.max(0, w * h * 4)) }; }
    isPointInPath() { return false; }
    isPointInStroke() { return false; }
}
for (const k of Object.keys(STYLE_DEFAULTS)) {
    Object.defineProperty(RecCtx.prototype, k, {
        get() { return this._s[k]; },
        set(v) {
            if (v === undefined || v === null) return;
            this._s[k] = v;
            if (k === 'fillStyle' || k === 'strokeStyle' || k === 'shadowColor') this.c.push(paintCmd(k, v));
            else this.c.push([k, typeof v === 'number' ? v : String(v)]);
        },
    });
}

// ───────────────────────────── core/renderer.js shim ─────────────────────────

export function drawVector(ctx, ox, oy, dx, dy, color, label = '') {
    const ex = ox + dx, ey = oy + dy, angle = Math.atan2(dy, dx), headLen = 10;
    ctx.strokeStyle = color; ctx.fillStyle = color; ctx.lineWidth = 2;
    ctx.beginPath(); ctx.moveTo(ox, oy); ctx.lineTo(ex, ey); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(ex, ey);
    ctx.lineTo(ex - headLen * Math.cos(angle - Math.PI / 6), ey - headLen * Math.sin(angle - Math.PI / 6));
    ctx.lineTo(ex - headLen * Math.cos(angle + Math.PI / 6), ey - headLen * Math.sin(angle + Math.PI / 6));
    ctx.closePath(); ctx.fill();
    if (label) {
        ctx.font = 'bold 10px Manrope, sans-serif'; ctx.fillStyle = color; ctx.textAlign = 'center';
        ctx.fillText(label, ex + Math.cos(angle) * 14, ey + Math.sin(angle) * 14);
    }
}
export function drawGrid(ctx, w, h) {
    ctx.strokeStyle = 'rgba(0, 99, 235, 0.03)'; ctx.lineWidth = 1;
    for (let x = 0; x < w; x += 30) { ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, h); ctx.stroke(); }
    for (let y = 0; y < h; y += 30) { ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(w, y); ctx.stroke(); }
}
export function showMathLine(text, type = 'text', highlight = false, seen) {
    if (seen) { if (seen.has(text)) return; seen.add(text); }
    __rw_math(String(text), String(type), !!highlight);
}
export function clearMathOverlay() { __rw_mathClear(); }
export class CanvasRenderer { constructor() {} beginFrame() { return {}; } endFrame() {} resize() {} destroy() {} }

// ───────────────────────────── timeline driver ─────────────────────────────

const simState = {
    showVectors: true, showGrid: false, showICR: false, mode: 'interactive',
    paused: false, speed: 1.0, time: 0, activePresetId: null,
};
let topic = null;
let preset = null;
let tween = null;     // the single active slider animation (app.js SliderAnimator)
let clockMs = 0;      // virtual wall clock for tweens

function animateSlider(key, from, to, durationMs, cb) {
    if (!preset) { if (cb) cb(); return; }
    const mp = preset.modes?.[simState.mode]?.params;
    if (!mp || !mp[key]) { if (cb) cb(); return; }
    tween = { mp, key, from: +from, to: +to, start: clockMs, dur: Math.max(1, +durationMs), cb };
    mp[key].value = +from;
}
function setICR(on) { simState.showICR = !!on; }

function switchPreset(id) {
    const p = topic.presets.find(x => x.id === id) || topic.presets[0];
    preset = p;
    simState.activePresetId = p.id;
    simState.time = 0;
    p.reset?.();
    if (p.id !== 'leaning-rod' && !(p.id || '').includes('icr')) simState.showICR = false;
}

function flatParams() {
    const mp = preset?.modes?.[simState.mode]?.params || {};
    const out = {};
    for (const k of Object.keys(mp)) out[k] = mp[k].value;
    return out;
}

/** Find the topic factory among the module's exports and build the manifest. */
export function load(mod) {
    let factory = null;
    for (const [k, v] of Object.entries(mod)) {
        if (typeof v === 'function' && /^create.*Topic$/.test(k)) { factory = v; break; }
    }
    if (!factory) for (const v of Object.values(mod)) if (typeof v === 'function') { factory = v; break; }
    if (factory) topic = factory(animateSlider, setICR);
    else topic = mod.default || mod;
    if (!topic || !Array.isArray(topic.presets)) throw new Error('lesson module exports no topic manifest');
    switchPreset(topic.presets[0].id);
    const steps = (tl) => (tl || []).map(s => ({
        text: String(s.text || ''), preset: s.preset || topic.presets[0].id, mode: s.mode || 'interactive',
        audioId: s.audioId === undefined ? null : String(s.audioId), audioDir: s.audioDir || topic.audioDir || 'audio/',
        pauseAfterMs: s.pauseAfterMs ?? 1200,
    }));
    // Derivations come as { presetId: [{title, math, desc}] } or as a list of
    // groups [{title, steps: [{text, math}]}] (keyed here by preset order).
    const item = d => ({ title: String(d.title ?? d.text ?? ''), math: String(d.math ?? ''), desc: String(d.desc ?? '') });
    const group = g => Array.isArray(g) ? g.map(item)
        : (g && Array.isArray(g.steps)) ? [{ title: String(g.title || ''), math: '', desc: '' }, ...g.steps.map(item)]
        : (g ? [item(g)] : []);
    const derivations = {};
    const D = topic.derivations;
    if (Array.isArray(D)) D.forEach((g, i) => { derivations['#' + i] = group(g); });
    else if (D && typeof D === 'object') for (const [k, v] of Object.entries(D)) derivations[k] = group(v);
    return JSON.stringify({
        id: topic.id || '', title: topic.title || '', subtitle: topic.subtitle || '',
        intro: steps(topic.introTimeline), lecture: steps(topic.lectureTimeline),
        presets: topic.presets.map(p => ({
            id: p.id, name: p.name || p.id, constraintEq: p.constraintEq || '', tip: p.tip || '',
            question: typeof p.question === 'string' ? p.question : '', level: p.level || '',
        })),
        derivations,
    });
}

/** Start step `index` of the 'intro' or 'lecture' timeline (app.js onStepStart). */
export function beginStep(part, index) {
    const tl = part === 'intro' ? topic.introTimeline : topic.lectureTimeline;
    const step = tl[index];
    switchPreset(step.preset);
    simState.mode = step.mode || 'interactive';
    clockMs = 0;
    if (tween) tween.start -= 1e12; // a tween left over from the previous step finishes immediately
    advanceTween();
    try { topic.activateNarrationStep?.(step); } catch (e) {}
    globalThis.__rwStep = step;
    if (step.action) step.action(() => {});
}

function advanceTween() {
    // Completing a tween may start the next one (callback chains), so loop.
    for (let guard = 0; tween && guard < 64; guard++) {
        const t = tween;
        const u = Math.min(1, Math.max(0, (clockMs - t.start) / t.dur));
        t.mp[t.key].value = t.from + (t.to - t.from) * u;
        if (u < 1) break;
        tween = null;
        if (t.cb) t.cb();
        if (tween === t) break;
    }
}

/** Advance one frame of `dt` seconds and return the frame's canvas commands. */
export function tick(dt, progress) {
    clockMs += dt * 1000;
    advanceTween();
    const params = flatParams();
    simState.time += dt;
    try { preset.step?.(dt, params, simState.mode, simState.time); } catch (e) { __rw_log('step: ' + e); }
    try { topic.setNarrationProgress?.(globalThis.__rwStep, progress); } catch (e) {}
    const ctx = new RecCtx();
    globalThis.__SIM_CTX = { ctx, simState };
    try {
        preset.draw(ctx, V_W, V_H, preset.state, params, simState, simState.time);
    } catch (e) { __rw_log('draw: ' + e); }
    return JSON.stringify(ctx.c);
}

/** Probe the math overlay of a step: lines with the word index where they appear. */
export function overlay(stepId, maxWords) {
    const out = [];
    if (!topic.updateMathOverlay) return '[]';
    globalThis.__rwLines = out;
    try { topic.updateMathOverlay('__runwee_reset__', 0); } catch (e) {}
    out.length = 0;
    const id = /^\d+$/.test(stepId) ? Number(stepId) : stepId;
    for (let wi = 0; wi <= maxWords; wi++) {
        globalThis.__rwWi = wi;
        try { topic.updateMathOverlay(id, wi); } catch (e) { __rw_log('overlay: ' + e); break; }
    }
    return JSON.stringify(out);
}

/** Show a preset in its default (interactive) state for `frames` frames and
 *  return the last frame's commands — used for previews and comparisons. */
export function previewPreset(id, frames, dt) {
    switchPreset(id);
    simState.mode = 'interactive';
    tween = null;
    let out = '[]';
    for (let i = 0; i < frames; i++) out = tick(dt, 0);
    return out;
}
