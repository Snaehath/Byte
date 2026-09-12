/**
 * Byte Energy Core Engine
 * 
 * 60+ FPS Living Quantum Energy Core
 * - 80% sphere + 20% living energy (90-95% circular base silhouette)
 * - 8-12 harmonic micro-perimeter wave synthesis (subtle ±3-6% breathing ripple, no amoeba bulges)
 * - High-lumen radiant white nucleus communicating machine intelligence
 * - Concentric gravitational particle orbits with 3D elliptical depth cueing
 * - Independent breathing outer aura with periodic cognitive expansion in Thinking state
 * - Audio-reactive surface micro-ripples and voice pulse modulation
 * - High-DPI auto-scaling for crisp 4K / Windows scaling displays
 */

class ByteLivingOrb {
  constructor(canvas) {
    if (!canvas) {
      console.error('ByteLivingOrb: canvas element is null or undefined!');
      return;
    }
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');

    this.width = 220;
    this.height = 175;
    this.setupHighDpi();

    this.centerX = this.width / 2;
    this.centerY = this.height / 2 - 2;

    // Clock
    this.time = 0;
    this.lastFrameTime = performance.now();

    // State management & continuous morphing
    this.currentState = 'idle';
    this.targetState = 'idle';
    this.stateTransition = 1.0;

    // Audio reactivity
    this.audioLevel = 0.0;
    this.targetAudioLevel = 0.0;

    // Gravitational orbital particles (3 distinct energy tracks)
    this.particles = [];
    this.initOrbitalParticles(28);

    // Energy Core visual presets per presence state
    this.presets = {
      idle: {
        baseRadius: 45,
        waveAmp: 1.8,            // ±4% deformation (96% circular)
        speed: 0.028,            // Slow, majestic breathing
        auraRadius: 64,
        auraPulseAmp: 4.0,
        coreBrightness: 0.95,
        colorCore: '#ffffff',
        colorInnerGlow: '#6c5ce7',
        colorMid: '#0984e3',
        colorEdge: '#00cec9',
        colorAura: 'rgba(9, 132, 227, 0.22)',
        particleSpeed: 0.55,     // Serene, slow cosmic drift (~30s orbit)
        particleTrackRadius: 48,
        compression: 0.0,
      },
      listening: {
        baseRadius: 48,
        waveAmp: 2.4,            // Subtle acoustic perturbation
        speed: 0.045,
        auraRadius: 72,
        auraPulseAmp: 6.0,
        coreBrightness: 1.0,
        colorCore: '#ffffff',
        colorInnerGlow: '#00d2d3',
        colorMid: '#0984e3',
        colorEdge: '#74b9ff',
        colorAura: 'rgba(0, 210, 211, 0.35)',
        particleSpeed: 0.85,     // Gentle attentiveness
        particleTrackRadius: 52,
        compression: 0.0,
      },
      thinking: {
        baseRadius: 42,          // Concentrated / slightly compressed core
        waveAmp: 2.0,            // Focused traveling perimeter wave
        speed: 0.08,             // Rapid cognitive energy flow
        auraRadius: 68,
        auraPulseAmp: 9.0,       // 1.5-second rhythmic brainwave pulse
        coreBrightness: 1.15,    // High-lumen intense white center
        colorCore: '#ffffff',
        colorInnerGlow: '#a29bfe',
        colorMid: '#6c5ce7',
        colorEdge: '#e056fd',
        colorAura: 'rgba(108, 92, 231, 0.38)',
        particleSpeed: 1.5,      // Controlled cognitive acceleration
        particleTrackRadius: 44, // Tight gravitational pull
        compression: 3.0,
      },
      speaking: {
        baseRadius: 47,
        waveAmp: 2.8,            // Voice ripple modulation
        speed: 0.05,
        auraRadius: 78,
        auraPulseAmp: 8.0,
        coreBrightness: 1.05,
        colorCore: '#ffffff',
        colorInnerGlow: '#00cec9',
        colorMid: '#00b894',
        colorEdge: '#55efc4',
        colorAura: 'rgba(0, 184, 148, 0.36)',
        particleSpeed: 0.95,     // Harmonic speech orbit
        particleTrackRadius: 54,
        compression: 0.0,
      },
      cancelled: {
        baseRadius: 38,
        waveAmp: 1.2,
        speed: 0.04,
        auraRadius: 54,
        auraPulseAmp: 2.0,
        coreBrightness: 0.75,
        colorCore: '#ffffff',
        colorInnerGlow: '#fab1a0',
        colorMid: '#e17055',
        colorEdge: '#d63031',
        colorAura: 'rgba(214, 48, 49, 0.25)',
        particleSpeed: 0.35,     // Calming deceleration
        particleTrackRadius: 42,
        compression: 4.0,
      },
    };

    this.currentParams = { ...this.presets.idle };

    // Bind and start rendering loop
    this.render = this.render.bind(this);
    requestAnimationFrame(this.render);
  }

  setupHighDpi() {
    const dpr = window.devicePixelRatio || 1;
    this.canvas.width = this.width * dpr;
    this.canvas.height = this.height * dpr;
    this.canvas.style.width = `${this.width}px`;
    this.canvas.style.height = `${this.height}px`;
    this.ctx.scale(dpr, dpr);
  }

  initOrbitalParticles(count = 20) {
    this.particles = [];
    // 3 orbital lanes: inner (lane 0), mid (lane 1), outer (lane 2)
    for (let i = 0; i < count; i++) {
      const lane = i % 3;
      const baseDistance = 44 + lane * 13 + (Math.random() - 0.5) * 6;
      // Base angular speed in radians per second (~0.20 - 0.35 rad/s)
      const baseRadPerSec = 0.20 + (2 - lane) * 0.07;
      this.particles.push({
        lane,
        angle: (i / count) * Math.PI * 2 + Math.random() * 0.4,
        distance: baseDistance,
        baseDistance,
        speed: baseRadPerSec * (Math.random() > 0.25 ? 1 : -1),
        size: 1.0 + Math.random() * 1.5,
        pulseOffset: Math.random() * Math.PI * 2,
        tilt: 0.72 + (lane * 0.05), // Elliptical 3D tilt
      });
    }
  }

  setState(newState) {
    if (this.presets[newState] && newState !== this.targetState) {
      this.targetState = newState;
      this.stateTransition = 0.0;
    }
  }

  setAudioLevel(level) {
    this.targetAudioLevel = Math.max(0.0, Math.min(1.0, level));
  }

  interpolateParams(from, to, t) {
    const lerp = (a, b, f) => a + (b - a) * f;
    return {
      baseRadius: lerp(from.baseRadius, to.baseRadius, t),
      waveAmp: lerp(from.waveAmp, to.waveAmp, t),
      speed: lerp(from.speed, to.speed, t),
      auraRadius: lerp(from.auraRadius, to.auraRadius, t),
      auraPulseAmp: lerp(from.auraPulseAmp, to.auraPulseAmp, t),
      coreBrightness: lerp(from.coreBrightness, to.coreBrightness, t),
      particleSpeed: lerp(from.particleSpeed, to.particleSpeed, t),
      particleTrackRadius: lerp(from.particleTrackRadius, to.particleTrackRadius, t),
      compression: lerp(from.compression, to.compression, t),
      colorCore: t > 0.5 ? to.colorCore : from.colorCore,
      colorInnerGlow: t > 0.5 ? to.colorInnerGlow : from.colorInnerGlow,
      colorMid: t > 0.5 ? to.colorMid : from.colorMid,
      colorEdge: t > 0.5 ? to.colorEdge : from.colorEdge,
      colorAura: t > 0.5 ? to.colorAura : from.colorAura,
    };
  }

  render(timestamp) {
    const dt = Math.min((timestamp - this.lastFrameTime) / 1000, 0.1);
    this.lastFrameTime = timestamp;
    this.time += dt;

    // Smooth state morphing
    if (this.stateTransition < 1.0) {
      this.stateTransition = Math.min(1.0, this.stateTransition + dt * 3.5);
      const fromPreset = this.presets[this.currentState] || this.presets.idle;
      const toPreset = this.presets[this.targetState] || this.presets.idle;
      this.currentParams = this.interpolateParams(fromPreset, toPreset, this.stateTransition);
      if (this.stateTransition >= 1.0) {
        this.currentState = this.targetState;
      }
    }

    // Audio reactivity smoothing
    this.audioLevel += (this.targetAudioLevel - this.audioLevel) * 0.28;

    // Clear viewport
    this.ctx.clearRect(0, 0, this.width, this.height);

    // 1. Draw back-half orbital particles (behind core depth illusion)
    this.drawParticles(dt, true);

    // 2. Draw dual-layer breathing aura
    this.drawLuminousAura();

    // 3. Draw high-lumen living Energy Core
    this.drawEnergyCore();

    // 4. Draw front-half orbital particles (in front of core)
    this.drawParticles(dt, false);

    requestAnimationFrame(this.render);
  }

  /**
   * Dual-Layer Atmospheric Aura:
   * Inner halo provides soft bloom; outer aura breathes independently
   * In 'thinking' state, pulses outward periodically every ~1.5s.
   */
  drawLuminousAura() {
    const params = this.currentParams;
    const isThinking = this.targetState === 'thinking' || this.currentState === 'thinking';
    
    // Cognitive pulse every ~1.5 seconds during thinking
    let thinkingPulse = 0.0;
    if (isThinking) {
      const cycle = (this.time % 1.5) / 1.5; // 0.0 to 1.0
      thinkingPulse = Math.sin(cycle * Math.PI) * params.auraPulseAmp * 1.5;
    }

    const audioPulse = this.audioLevel * 14.0;
    const breathingCycle = Math.sin(this.time * 1.8) * params.auraPulseAmp;
    const currentAuraRadius = Math.max(20, params.auraRadius + breathingCycle + thinkingPulse + audioPulse);

    this.ctx.save();

    // 1. Outer Ethereal Atmosphere
    const outerGrad = this.ctx.createRadialGradient(
      this.centerX,
      this.centerY,
      params.baseRadius * 0.7,
      this.centerX,
      this.centerY,
      currentAuraRadius
    );
    outerGrad.addColorStop(0, params.colorAura);
    outerGrad.addColorStop(0.65, params.colorAura.replace(/[\d\.]+\)$/, '0.08)'));
    outerGrad.addColorStop(1, 'rgba(0, 0, 0, 0)');

    this.ctx.beginPath();
    this.ctx.arc(this.centerX, this.centerY, currentAuraRadius, 0, Math.PI * 2);
    this.ctx.fillStyle = outerGrad;
    this.ctx.fill();

    // 2. Inner Energy Sheen (tight corona hugging the nucleus)
    const innerCoronaRadius = params.baseRadius + 8.0 + (isThinking ? Math.sin(this.time * 6.0) * 2.0 : 0);
    const coronaGrad = this.ctx.createRadialGradient(
      this.centerX,
      this.centerY,
      params.baseRadius * 0.4,
      this.centerX,
      this.centerY,
      innerCoronaRadius
    );
    coronaGrad.addColorStop(0, 'rgba(255, 255, 255, 0.25)');
    coronaGrad.addColorStop(0.5, params.colorInnerGlow);
    coronaGrad.addColorStop(1, 'rgba(0, 0, 0, 0)');

    this.ctx.beginPath();
    this.ctx.arc(this.centerX, this.centerY, innerCoronaRadius, 0, Math.PI * 2);
    this.ctx.fillStyle = coronaGrad;
    this.ctx.fill();

    this.ctx.restore();
  }

  /**
   * Living Energy Core:
   * 80% sphere + 20% living energy.
   * High-harmonic subtle wave synthesis (8-12 smooth micro-waves, ±3-6% radius deviation).
   * Luminous radiant white intelligence center, rich cyan/violet depth, no amoeba bulges.
   */
  drawEnergyCore() {
    const params = this.currentParams;
    const pointsCount = 64;
    const angleStep = (Math.PI * 2) / pointsCount;
    const isThinking = this.targetState === 'thinking' || this.currentState === 'thinking';

    // Base radius with subtle compression in thinking/cancelled states
    const breathing = Math.sin(this.time * 2.2) * 1.2;
    const effectiveRadius = Math.max(12, params.baseRadius - params.compression + breathing);

    // Audio reactivity micro-ripples
    const audioRipple = this.audioLevel * 3.5;

    // Synthesize perimeter points
    const points = [];
    for (let i = 0; i < pointsCount; i++) {
      const angle = i * angleStep;

      // 8 to 12 smooth harmonic waves (subtle ±3% to 6% ripple, never random amoeba blobs)
      const wave1 = Math.sin(angle * 8.0 + this.time * 2.6) * (params.waveAmp * 0.55);
      const wave2 = Math.cos(angle * 10.0 - this.time * 2.1) * (params.waveAmp * 0.35);
      const wave3 = Math.sin(angle * 12.0 + this.time * 3.4) * (params.waveAmp * 0.20);

      // Thinking: a focused energy wave sweeping smoothly around the perimeter
      let travelingWave = 0.0;
      if (isThinking) {
        travelingWave = Math.sin(angle * 4.0 - this.time * 5.5) * 1.6;
      }

      // Acoustic vibration during voice input / output
      const soundWave = Math.sin(angle * 14.0 + this.time * 16.0) * audioRipple;

      const r = effectiveRadius + wave1 + wave2 + wave3 + travelingWave + soundWave;
      points.push({
        x: this.centerX + Math.cos(angle) * r,
        y: this.centerY + Math.sin(angle) * r,
      });
    }

    this.ctx.save();
    this.ctx.beginPath();

    // Silky smooth closed spline via quadratic midpoints
    const len = points.length;
    let midX = (points[0].x + points[1].x) / 2;
    let midY = (points[0].y + points[1].y) / 2;
    this.ctx.moveTo(midX, midY);

    for (let i = 1; i < len; i++) {
      const nextIdx = (i + 1) % len;
      midX = (points[i].x + points[nextIdx].x) / 2;
      midY = (points[i].y + points[nextIdx].y) / 2;
      this.ctx.quadraticCurveTo(points[i].x, points[i].y, midX, midY);
    }
    midX = (points[0].x + points[1].x) / 2;
    midY = (points[0].y + points[1].y) / 2;
    this.ctx.quadraticCurveTo(points[0].x, points[0].y, midX, midY);
    this.ctx.closePath();

    // Radiant Intelligence Nucleus (pure intense white core -> luminous mid -> deep electric shell)
    const focalShiftX = Math.sin(this.time * 1.2) * 2.5;
    const focalShiftY = Math.cos(this.time * 1.2) * 2.5;
    const coreGrad = this.ctx.createRadialGradient(
      this.centerX + focalShiftX,
      this.centerY + focalShiftY,
      1,
      this.centerX,
      this.centerY,
      effectiveRadius + 4
    );

    // Brilliant white center communicates AI consciousness
    coreGrad.addColorStop(0, '#ffffff');
    coreGrad.addColorStop(0.22, '#ffffff');
    coreGrad.addColorStop(0.42, params.colorInnerGlow);
    coreGrad.addColorStop(0.76, params.colorMid);
    coreGrad.addColorStop(0.96, params.colorEdge);
    coreGrad.addColorStop(1.0, params.colorInnerGlow);

    this.ctx.fillStyle = coreGrad;
    this.ctx.shadowBlur = (16 + this.audioLevel * 14) * params.coreBrightness;
    this.ctx.shadowColor = params.colorInnerGlow;
    this.ctx.fill();

    // Delicate translucent luminous rim (thin crystalline energy shell)
    this.ctx.lineWidth = 1.2;
    this.ctx.strokeStyle = `rgba(255, 255, 255, ${Math.min(0.85, 0.45 * params.coreBrightness + this.audioLevel * 0.3)})`;
    this.ctx.stroke();

    // Intense high-lumen center spark
    const sparkRadius = Math.max(3, effectiveRadius * 0.22);
    const sparkGrad = this.ctx.createRadialGradient(
      this.centerX + focalShiftX * 0.5,
      this.centerY + focalShiftY * 0.5,
      0,
      this.centerX,
      this.centerY,
      sparkRadius
    );
    sparkGrad.addColorStop(0, `rgba(255, 255, 255, ${Math.min(1.0, 0.95 * params.coreBrightness)})`);
    sparkGrad.addColorStop(0.7, 'rgba(255, 255, 255, 0.6)');
    sparkGrad.addColorStop(1, 'rgba(255, 255, 255, 0)');

    this.ctx.beginPath();
    this.ctx.arc(this.centerX, this.centerY, sparkRadius, 0, Math.PI * 2);
    this.ctx.fillStyle = sparkGrad;
    this.ctx.fill();

    this.ctx.restore();
  }

  /**
   * Gravitational Orbital Particles:
   * Particles orbit in 3 distinct elliptical tracks with front/back 3D depth cueing.
   * Thinking state accelerates orbit velocity and tightens the track.
   */
  drawParticles(dt, isBackHalf) {
    const params = this.currentParams;
    const isThinking = this.targetState === 'thinking' || this.currentState === 'thinking';

    this.ctx.save();

    for (const p of this.particles) {
      // Accelerate orbit in thinking state or on audio response (scaled by dt for frame-rate independence)
      const speedMultiplier = params.particleSpeed * (isThinking ? 1.4 : 1.0) * (1.0 + this.audioLevel * 0.8);
      p.angle += p.speed * speedMultiplier * dt;

      // Elliptical coordinate projection
      const sinA = Math.sin(p.angle);
      const cosA = Math.cos(p.angle);

      // Distinguish front half from back half for 3D occlusion
      const inBack = sinA < 0;
      if (inBack !== isBackHalf) continue;

      // Gravitational pull: tracks tighten during thinking
      const trackDist = params.particleTrackRadius + (p.lane * 11) + Math.sin(this.time * 1.2 + p.pulseOffset) * 2.0;
      const px = this.centerX + cosA * trackDist;
      const py = this.centerY + sinA * (trackDist * p.tilt);

      // Depth modulation: front particles are brighter and slightly larger
      const depthAlpha = inBack ? 0.35 : 0.85;
      const alpha = depthAlpha * (0.6 + Math.sin(this.time * 3.5 + p.pulseOffset) * 0.4);
      const size = inBack ? p.size * 0.75 : p.size * (1.0 + this.audioLevel * 0.4);

      this.ctx.beginPath();
      this.ctx.arc(px, py, Math.max(0.6, size), 0, Math.PI * 2);
      this.ctx.fillStyle = `rgba(255, 255, 255, ${Math.max(0, Math.min(1, alpha)).toFixed(2)})`;
      this.ctx.shadowBlur = inBack ? 2 : 6;
      this.ctx.shadowColor = params.colorInnerGlow;
      this.ctx.fill();
    }

    this.ctx.restore();
  }
}

// Attach to window global
if (typeof window !== 'undefined') {
  window.ByteLivingOrb = ByteLivingOrb;
}
