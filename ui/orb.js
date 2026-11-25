/**
 * Byte Orb
 *
 * A perfectly round, still sphere filled with slowly churning liquid color —
 * rendered with a WebGL fragment shader (2D canvas fallback when WebGL is
 * unavailable). Each presence state blends to its own palette and motion,
 * and the sphere responds to live audio level.
 */

const ORB_STATES = {
  // speed: how fast the liquid flows; swirl: how strongly it churns.
  idle:      { colors: ['#1d3bff', '#62c6ff', '#e9f3ff'], speed: 0.14, swirl: 0.15, glow: 0.35, scale: 1.00 },
  listening: { colors: ['#0a7cff', '#3fe0ff', '#f1fdff'], speed: 0.26, swirl: 0.35, glow: 0.55, scale: 1.03 },
  thinking:  { colors: ['#5b21f6', '#b28dff', '#ffd6fb'], speed: 0.42, swirl: 0.85, glow: 0.50, scale: 0.97 },
  speaking:  { colors: ['#1f4dff', '#3ad2ff', '#b8ffe6'], speed: 0.32, swirl: 0.45, glow: 0.60, scale: 1.02 },
  cancelled: { colors: ['#3c4a63', '#8796ae', '#dfe6f0'], speed: 0.08, swirl: 0.10, glow: 0.18, scale: 0.96 },
  error:     { colors: ['#b4232c', '#ff7a7a', '#ffe3e3'], speed: 0.12, swirl: 0.12, glow: 0.35, scale: 0.97 },
};

const VERT = `
attribute vec2 aPos;
void main() { gl_Position = vec4(aPos, 0.0, 1.0); }
`;

const FRAG = `
precision highp float;
uniform vec2  uRes;
uniform float uPhase;
uniform float uSwirl;
uniform float uLevel;
uniform float uGlow;
uniform float uScale;
uniform vec3  uC1;
uniform vec3  uC2;
uniform vec3  uC3;

vec3 hash3(vec2 p) {
  vec3 q = vec3(dot(p, vec2(127.1, 311.7)), dot(p, vec2(269.5, 183.3)), dot(p, vec2(419.2, 371.9)));
  return fract(sin(q) * 43758.5453);
}

float noise(vec2 p) {
  vec2 i = floor(p);
  vec2 f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  float a = hash3(i).x;
  float b = hash3(i + vec2(1.0, 0.0)).x;
  float c = hash3(i + vec2(0.0, 1.0)).x;
  float d = hash3(i + vec2(1.0, 1.0)).x;
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

float fbm(vec2 p) {
  float v = 0.0;
  float a = 0.5;
  mat2 r = mat2(0.8, -0.6, 0.6, 0.8);
  for (int i = 0; i < 4; i++) {
    v += a * noise(p);
    p = r * p * 2.02 + 7.3;
    a *= 0.5;
  }
  return v;
}

void main() {
  float px = 2.0 / min(uRes.x, uRes.y);
  vec2 p = (gl_FragCoord.xy - 0.5 * uRes) * px;   // -1..1 square space

  float breathe = 0.012 * sin(uPhase * 3.2);
  float R = 0.74 * uScale * (1.0 + breathe + uLevel * 0.075);
  float r = length(p);

  // Outer halo (premultiplied, additive)
  float halo = exp(-max(r - R, 0.0) * 9.0) * smoothstep(R - 0.02, R + 0.01, r);
  halo *= 1.0 - smoothstep(0.86, 0.99, r);   // fade to zero before the canvas edge (no square outline)
  vec3 haloCol = mix(uC1, uC2, 0.6) * halo * (uGlow + uLevel * 0.6) * 0.55;
  float haloA = halo * (uGlow + uLevel * 0.6) * 0.45;

  float inside = 1.0 - smoothstep(R - px * 1.2, R + px * 0.6, r);
  if (inside <= 0.0) {
    gl_FragColor = vec4(haloCol, haloA);
    return;
  }

  vec2 q = p / R;
  float z = sqrt(max(0.0, 1.0 - dot(q, q)));

  // Slight lens distortion so the fluid feels like it lives on a sphere
  vec2 uv = q * (1.0 - 0.28 * (1.0 - z));
  float t = uPhase;

  // Voice makes soft ripples travel outward through the liquid.
  float rq = length(q);
  uv += q * sin(rq * 9.0 - t * 14.0) * uLevel * 0.035;

  // Two layers of flow drifting in different directions (no rotation):
  // the second is warped by the first, so colors fold into each other.
  vec2 w1 = vec2(fbm(uv * 1.2 + vec2(0.0, t * 0.30)),
                 fbm(uv * 1.2 + vec2(5.2, -t * 0.24)));
  vec2 w2 = vec2(fbm(uv * 1.7 + w1 * 1.6 + vec2(t * 0.18, 1.7)),
                 fbm(uv * 1.7 + w1 * 1.6 + vec2(8.3, -t * 0.14)));
  float churn = 1.2 + uSwirl * 2.2 + uLevel * 1.6;
  float n1 = fbm(uv * 1.05 + w2 * churn);
  float n2 = fbm(uv * 0.85 - w1 * (1.0 + uSwirl) + vec2(-t * 0.06, 3.0 + t * 0.05));

  vec3 col = mix(uC1, uC2, smoothstep(0.28, 0.72, n1));
  col = mix(col, uC3, smoothstep(0.52, 0.86, n2) * 0.85);

  // Soft sphere shading
  vec3 N = normalize(vec3(q, z));
  vec3 L = normalize(vec3(-0.45, 0.55, 0.75));
  float diff = 0.62 + 0.38 * max(dot(N, L), 0.0);
  col *= diff;

  // Inner light that rises with voice energy
  col += uC3 * pow(z, 3.0) * (0.10 + uLevel * 0.35);

  // Fresnel rim
  float fres = pow(1.0 - z, 3.0);
  col = mix(col, uC2 * 1.15 + 0.1, fres * 0.55);

  // Gentle specular sheen
  vec3 H = normalize(L + vec3(0.0, 0.0, 1.0));
  float spec = pow(max(dot(N, H), 0.0), 38.0);
  col += vec3(1.0) * spec * 0.35;

  col = clamp(col, 0.0, 1.0);
  gl_FragColor = vec4(col * inside + haloCol * (1.0 - inside), inside + haloA * (1.0 - inside));
}
`;

function hexToRgb(hex) {
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}

class ByteLivingOrb {
  constructor(canvas) {
    if (!canvas) throw new Error('ByteLivingOrb: canvas is required');
    this.canvas = canvas;
    this.state = 'idle';
    this.phase = Math.random() * 100;
    this.level = 0;
    this.targetLevel = 0;
    this.reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;

    const s = ORB_STATES.idle;
    this.current = {
      c: s.colors.map(hexToRgb),
      speed: s.speed, swirl: s.swirl, glow: s.glow, scale: s.scale,
    };
    this.lastTime = performance.now();

    this.gl = this.initGl();
    if (!this.gl) this.ctx = canvas.getContext('2d');
    this.resize();
    window.addEventListener('resize', () => this.resize());
    this.frame = this.frame.bind(this);
    requestAnimationFrame(this.frame);
  }

  initGl() {
    let gl = null;
    try {
      gl = this.canvas.getContext('webgl', { premultipliedAlpha: true, alpha: true, antialias: true });
    } catch (_) { /* fall through to 2D */ }
    if (!gl) return null;

    const compile = (type, src) => {
      const sh = gl.createShader(type);
      gl.shaderSource(sh, src);
      gl.compileShader(sh);
      if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
        console.error('Orb shader error:', gl.getShaderInfoLog(sh));
        return null;
      }
      return sh;
    };
    const vs = compile(gl.VERTEX_SHADER, VERT);
    const fs = compile(gl.FRAGMENT_SHADER, FRAG);
    if (!vs || !fs) return null;

    const prog = gl.createProgram();
    gl.attachShader(prog, vs);
    gl.attachShader(prog, fs);
    gl.linkProgram(prog);
    if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) {
      console.error('Orb program link error:', gl.getProgramInfoLog(prog));
      return null;
    }
    gl.useProgram(prog);

    const buf = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, buf);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
    const loc = gl.getAttribLocation(prog, 'aPos');
    gl.enableVertexAttribArray(loc);
    gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);

    this.u = {};
    for (const name of ['uRes', 'uPhase', 'uSwirl', 'uLevel', 'uGlow', 'uScale', 'uC1', 'uC2', 'uC3']) {
      this.u[name] = gl.getUniformLocation(prog, name);
    }
    gl.clearColor(0, 0, 0, 0);
    return gl;
  }

  resize() {
    const dpr = Math.min(window.devicePixelRatio || 1, 2.5);
    const w = Math.max(1, Math.round((this.canvas.clientWidth || 172) * dpr));
    const h = Math.max(1, Math.round((this.canvas.clientHeight || 172) * dpr));
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    if (this.gl) this.gl.viewport(0, 0, w, h);
  }

  setState(state) {
    this.state = ORB_STATES[state] ? state : 'idle';
  }

  setAudioLevel(level) {
    const v = Number(level);
    this.targetLevel = Number.isFinite(v) ? Math.max(0, Math.min(1, v)) : 0;
  }

  frame(now) {
    const dt = Math.min(0.05, (now - this.lastTime) / 1000);
    this.lastTime = now;

    const target = ORB_STATES[this.state];
    const k = 1 - Math.exp(-dt * 3.5);
    const cur = this.current;
    for (let i = 0; i < 3; i++) {
      const tc = hexToRgb(target.colors[i]);
      for (let j = 0; j < 3; j++) cur.c[i][j] += (tc[j] - cur.c[i][j]) * k;
    }
    cur.speed += (target.speed - cur.speed) * k;
    cur.swirl += (target.swirl - cur.swirl) * k;
    cur.glow += (target.glow - cur.glow) * k;
    cur.scale += (target.scale - cur.scale) * k;

    const attack = this.targetLevel > this.level ? 18 : 6;
    this.level += (this.targetLevel - this.level) * (1 - Math.exp(-dt * attack));

    const motion = this.reducedMotion ? 0.25 : 1;
    this.phase += dt * (cur.speed + this.level * 0.6) * motion;

    if (this.gl) this.drawGl(); else this.draw2d();
    requestAnimationFrame(this.frame);
  }

  drawGl() {
    const gl = this.gl;
    const c = this.current;
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.uniform2f(this.u.uRes, this.canvas.width, this.canvas.height);
    gl.uniform1f(this.u.uPhase, this.phase);
    gl.uniform1f(this.u.uSwirl, c.swirl);
    gl.uniform1f(this.u.uLevel, this.level);
    gl.uniform1f(this.u.uGlow, c.glow);
    gl.uniform1f(this.u.uScale, c.scale);
    gl.uniform3fv(this.u.uC1, c.c[0]);
    gl.uniform3fv(this.u.uC2, c.c[1]);
    gl.uniform3fv(this.u.uC3, c.c[2]);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
  }

  draw2d() {
    const ctx = this.ctx;
    const { width: w, height: h } = this.canvas;
    const c = this.current;
    const rgb = (v, a = 1) => `rgba(${v.map((x) => Math.round(x * 255)).join(',')},${a})`;
    const cx = w / 2;
    const cy = h / 2;
    const R = Math.min(w, h) * 0.37 * c.scale * (1 + 0.012 * Math.sin(this.phase * 3.2) + this.level * 0.075);

    ctx.clearRect(0, 0, w, h);
    const halo = ctx.createRadialGradient(cx, cy, R * 0.9, cx, cy, R * 1.35);
    halo.addColorStop(0, rgb(c.c[1], 0.35 * (c.glow + this.level)));
    halo.addColorStop(1, rgb(c.c[1], 0));
    ctx.fillStyle = halo;
    ctx.fillRect(0, 0, w, h);

    ctx.save();
    ctx.beginPath();
    ctx.arc(cx, cy, R, 0, Math.PI * 2);
    ctx.clip();
    ctx.fillStyle = rgb(c.c[0]);
    ctx.fillRect(cx - R, cy - R, R * 2, R * 2);
    const blobs = [[c.c[1], 0.9, 0], [c.c[2], 0.6, 2.1], [c.c[1], 0.7, 4.2]];
    for (const [col, size, off] of blobs) {
      const a = this.phase * (0.6 + c.swirl) + off;
      const bx = cx + Math.sin(a * 0.7 + off) * R * 0.32;
      const by = cy + Math.sin(a * 1.1 + off * 1.7) * R * 0.32;
      const g = ctx.createRadialGradient(bx, by, 0, bx, by, R * size);
      g.addColorStop(0, rgb(col, 0.85));
      g.addColorStop(1, rgb(col, 0));
      ctx.fillStyle = g;
      ctx.fillRect(cx - R, cy - R, R * 2, R * 2);
    }
    const shade = ctx.createRadialGradient(cx - R * 0.35, cy - R * 0.4, R * 0.1, cx, cy, R);
    shade.addColorStop(0, 'rgba(255,255,255,0.18)');
    shade.addColorStop(0.7, 'rgba(0,0,0,0)');
    shade.addColorStop(1, 'rgba(0,0,0,0.22)');
    ctx.fillStyle = shade;
    ctx.fillRect(cx - R, cy - R, R * 2, R * 2);
    ctx.restore();
  }
}

window.ByteLivingOrb = ByteLivingOrb;
