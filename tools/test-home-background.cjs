const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const source = fs.readFileSync(path.join(__dirname, "../website/assets/javascripts/home.js"), "utf8")
  .replace("  const dots = makeDots();", "  const dots = makeDots();\n  window.homeAudit = { meridians, parallels, edges, dots };");
let frame = null;
const frames = [];
const raf = new Map();
const windowListeners = new Map();
const documentListeners = new Map();
const mediaListeners = new Map();
let nextFrame = 0;
let hidden = false;
let reduceMotion = false;

const context = {
  clearRect() {
    frame = { strokes: [], points: [], dots: [] };
    frames.push(frame);
  },
  setTransform() {},
  beginPath() {},
  moveTo(x, y) { frame.points.push([x, y]); },
  lineTo(x, y) { frame.points.push([x, y]); },
  stroke() { frame.strokes.push(this.strokeStyle); },
  arc(x, y, radius) { frame.dots.push({ x, y, radius, color: this.fillStyle }); },
  fill() {},
};
const canvas = {
  width: 0,
  height: 0,
  style: {},
  getContext() { return context; },
};
const window = {
  innerWidth: 1265,
  innerHeight: 790,
  devicePixelRatio: 2.5,
  addEventListener(name, handler) { windowListeners.set(name, handler); },
};
const document = {
  get hidden() { return hidden; },
  querySelector(selector) {
    assert.equal(selector, ".ond-home__background");
    return canvas;
  },
  addEventListener(name, handler) { documentListeners.set(name, handler); },
};
const media = {
  get matches() { return reduceMotion; },
  addEventListener(name, handler) { mediaListeners.set(name, handler); },
};

vm.runInNewContext(source, {
  Math,
  Number,
  Map,
  document,
  window,
  matchMedia() { return media; },
  requestAnimationFrame(handler) {
    const id = ++nextFrame;
    raf.set(id, handler);
    return id;
  },
  cancelAnimationFrame(id) { raf.delete(id); },
});

function runFrame(timestamp) {
  assert.equal(raf.size, 1, "exactly one animation frame should be queued");
  const [id, callback] = raf.entries().next().value;
  raf.delete(id);
  callback(timestamp);
}

function snapshot() {
  return JSON.stringify(frame);
}

function assertFiniteFrame() {
  for (const [x, y] of frame.points) assert.ok(Number.isFinite(x) && Number.isFinite(y));
  for (const dot of frame.dots) {
    assert.ok(Number.isFinite(dot.x) && Number.isFinite(dot.y));
    assert.equal(dot.radius, 1.7);
    assert.equal(dot.color, "rgba(237,56,53,.62)");
  }
}

assert.equal(canvas.width, 2530, "DPR should be clamped to 2");
assert.equal(window.homeAudit.meridians, 24);
assert.equal(window.homeAudit.parallels.length, 11);
assert.equal(window.homeAudit.edges.length, 35);
assert.equal(window.homeAudit.dots.length, 18);
const parallels = window.homeAudit.parallels;
const dotParallels = [0, 2, 4, 6, 8, 10, 5, 7];
for (const [index, dot] of window.homeAudit.dots.entries()) {
  for (const seconds of [0, 30]) {
    const [x, y, z] = dot.point(seconds);
    assert.ok(Math.abs(Math.hypot(x, y, z) - 1) < 1e-12, "dots should lie on the unit sphere");
    if (index < 10) {
      const longitude = (index * 2 * Math.PI) / 24;
      assert.ok(Math.abs(x * Math.cos(longitude) - z * Math.sin(longitude)) < 1e-12, "meridian dots should stay on their great circle");
    } else {
      assert.ok(Math.abs(y - Math.sin(parallels[dotParallels[index - 10]])) < 1e-12, "parallel dots should stay on a grid latitude");
    }
  }
}
assert.equal(frame.strokes.length, 35, "24 meridians and 11 parallels should be drawn");
assert.ok(frame.strokes.every((color) => color === "rgba(101,99,94,.16)"), "UV grid should remain gray");
assert.ok(frame.dots.length > 0 && frame.dots.length <= 18);
assertFiniteFrame();
assert.equal(raf.size, 1);

runFrame(1000);
const beforeMotion = snapshot();
runFrame(1016);
const afterMotion = snapshot();
assert.notEqual(afterMotion, beforeMotion, "camera and particles should move");
assertFiniteFrame();
assert.equal(raf.size, 1, "animation should not queue duplicate frames");

hidden = true;
documentListeners.get("visibilitychange")();
assert.equal(raf.size, 0, "hidden pages should stop the animation");
const beforePause = snapshot();
hidden = false;
documentListeners.get("visibilitychange")();
runFrame(90000);
assert.equal(snapshot(), beforePause, "resuming should not jump across the hidden interval");
runFrame(90016);
assert.notEqual(snapshot(), beforePause, "animation should continue after resuming");

const beforeResize = snapshot();
windowListeners.get("resize")();
assert.equal(snapshot(), beforeResize, "resize should preserve the current pose and particle phase");

reduceMotion = true;
mediaListeners.get("change")();
assert.equal(raf.size, 0, "reduced motion should stop the animation");
assert.equal(snapshot(), beforeResize, "reduced motion should keep a still frame");
assertFiniteFrame();

console.log("Home background animation checks passed.");
