(() => {
  const canvas = document.querySelector(".ond-home__background");
  if (!canvas) return;

  const context = canvas.getContext("2d", { alpha: true });
  const reducedMotion = matchMedia("(prefers-reduced-motion: reduce)");
  const twoPi = Math.PI * 2;
  const meridians = 24;
  const parallels = Array.from({ length: 11 }, (_, index) => ((index - 5) * Math.PI) / 12);
  const near = 0.16;
  const angularSpeed = twoPi / 95;
  const edges = makeSphereGrid();
  const dots = makeDots();
  let orientation = initialOrientation();
  let visualTime = 0;
  let width = 0;
  let height = 0;
  let frame = 0;
  let lastTimestamp = null;
  let pixelRatio = 1;

  function makeSphereGrid() {
    const lines = [];
    const steps = 96;

    for (let index = 0; index < meridians; index++) {
      const longitude = (twoPi * index) / meridians;
      const points = [];
      for (let step = 0; step <= steps; step++) {
        const latitude = -Math.PI / 2 + (Math.PI * step) / steps;
        const radius = Math.cos(latitude);
        points.push([radius * Math.sin(longitude), Math.sin(latitude), radius * Math.cos(longitude)]);
      }
      lines.push({ points });
    }

    for (const latitude of parallels) {
      const radius = Math.cos(latitude);
      const points = [];
      for (let step = 0; step <= steps; step++) {
        const longitude = (twoPi * step) / steps;
        points.push([radius * Math.sin(longitude), Math.sin(latitude), radius * Math.cos(longitude)]);
      }
      lines.push({ points });
    }
    return lines;
  }

  function makeDots() {
    const dots = Array.from({ length: 10 }, (_, index) => {
      const longitude = (index * twoPi) / meridians;
      const phase = (index * 2.399963229728653) % twoPi;
      const speed = 0.016 + (index % 5) * 0.0035;
      return {
        point: (seconds) => {
          const latitude = phase + seconds * speed;
          return [Math.cos(latitude) * Math.sin(longitude), Math.sin(latitude), Math.cos(latitude) * Math.cos(longitude)];
        },
      };
    });
    const parallelIndices = [0, 2, 4, 6, 8, 10, 5, 7];
    for (const [index, parallelIndex] of parallelIndices.entries()) {
      const latitude = parallels[parallelIndex];
      const phase = (index * 2.177) % twoPi;
      const speed = 0.018 + (index % 5) * 0.0034;
      const radius = Math.cos(latitude);
      dots.push({
        point: (seconds) => {
          const longitude = phase + seconds * speed;
          return [radius * Math.sin(longitude), Math.sin(latitude), radius * Math.cos(longitude)];
        },
      });
    }
    return dots;
  }

  function resize() {
    width = window.innerWidth;
    height = window.innerHeight;
    pixelRatio = Math.min(window.devicePixelRatio || 1, 2);
    canvas.width = Math.round(width * pixelRatio);
    canvas.height = Math.round(height * pixelRatio);
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
    render();
  }

  function multiply(a, b) {
    const [aw, ax, ay, az] = a;
    const [bw, bx, by, bz] = b;
    return [
      aw * bw - ax * bx - ay * by - az * bz,
      aw * bx + ax * bw + ay * bz - az * by,
      aw * by - ax * bz + ay * bw + az * bx,
      aw * bz + ax * by - ay * bx + az * bw,
    ];
  }

  function normalizeQuaternion(q) {
    const length = Math.hypot(...q);
    return q.map((value) => value / length);
  }

  function initialOrientation() {
    const yaw = 0.39 / 2;
    const pitch = 0.24 / 2;
    const roll = 0.045 / 2;
    const qYaw = [Math.cos(yaw), 0, Math.sin(yaw), 0];
    const qPitch = [Math.cos(pitch), Math.sin(pitch), 0, 0];
    const qRoll = [Math.cos(roll), 0, 0, Math.sin(roll)];
    return normalizeQuaternion(multiply(qRoll, multiply(qPitch, qYaw)));
  }

  function cameraAxis(time) {
    const axis = [0.55 * Math.cos(time / 23), 0.8 + 0.25 * Math.sin(time / 31), 0.55 * Math.sin(time / 23)];
    const length = Math.hypot(...axis);
    return axis.map((value) => value / length);
  }

  function advanceOrientation(seconds) {
    const axis = cameraAxis(visualTime);
    const halfAngle = (angularSpeed * seconds) / 2;
    const scale = Math.sin(halfAngle);
    const delta = normalizeQuaternion([Math.cos(halfAngle), axis[0] * scale, axis[1] * scale, axis[2] * scale]);
    orientation = normalizeQuaternion(multiply(delta, orientation));
    visualTime += seconds;
  }

  function rotationMatrix([w, x, y, z]) {
    return [
      1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w),
      2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w),
      2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y),
    ];
  }

  function rotate([x, y, z], matrix) {
    return [matrix[0] * x + matrix[1] * y + matrix[2] * z, matrix[3] * x + matrix[4] * y + matrix[5] * z, matrix[6] * x + matrix[7] * y + matrix[8] * z];
  }

  function project(point, focal, centerX, centerY) {
    return [centerX + (point[0] * focal) / point[2], centerY - (point[1] * focal) / point[2]];
  }

  function render() {
    context.clearRect(0, 0, width, height);
    if (!width || !height) return;
    const matrix = rotationMatrix(orientation);
    const focal = Math.min(width, height) * 0.54;
    const centerX = width / 2;
    const centerY = height / 2;

    for (const edge of edges) {
      const transformed = edge.points.map((point) => rotate(point, matrix));
      context.beginPath();
      context.strokeStyle = "rgba(101,99,94,.16)";
      context.lineWidth = 0.8;
      for (let index = 1; index < transformed.length; index++) {
        let from = transformed[index - 1];
        let to = transformed[index];
        const fromInside = from[2] >= near;
        const toInside = to[2] >= near;
        if (!fromInside && !toInside) continue;
        if (fromInside !== toInside) {
          const amount = (near - from[2]) / (to[2] - from[2]);
          const intersection = from.map((value, axis) => value + (to[axis] - value) * amount);
          if (fromInside) to = intersection;
          else from = intersection;
        }
        const start = project(from, focal, centerX, centerY);
        const end = project(to, focal, centerX, centerY);
        context.moveTo(start[0], start[1]);
        context.lineTo(end[0], end[1]);
      }
      context.stroke();
    }

    context.fillStyle = "rgba(237,56,53,.62)";
    for (const dot of dots) {
      const point = rotate(dot.point(visualTime), matrix);
      if (point[2] < near) continue;
      const [x, y] = project(point, focal, centerX, centerY);
      if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
      context.beginPath();
      context.arc(x, y, 1.7, 0, twoPi);
      context.fill();
    }
  }

  function animate(now) {
    if (lastTimestamp !== null) advanceOrientation((now - lastTimestamp) / 1000);
    lastTimestamp = now;
    render();
    frame = requestAnimationFrame(animate);
  }

  function stop() {
    cancelAnimationFrame(frame);
    frame = 0;
    lastTimestamp = null;
  }

  function updateMotion() {
    stop();
    if (reducedMotion.matches || document.hidden) {
      render();
      return;
    }
    frame = requestAnimationFrame(animate);
  }

  window.addEventListener("resize", resize, { passive: true });
  document.addEventListener("visibilitychange", updateMotion);
  reducedMotion.addEventListener("change", updateMotion);
  resize();
  updateMotion();
})();
