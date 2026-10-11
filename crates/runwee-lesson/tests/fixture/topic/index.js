import { drawVector } from '../core/renderer.js';
import { updateMathOverlay } from './mathOverlay.js';
import config from './config.json';

export function createFixtureTopic(animateSlider) {
    const ball = {
        id: 'ball', name: 'Rolling ball', constraintEq: 'x = vt',
        modes: { interactive: { params: { v: { value: 0 } } } },
        state: { x: 0 },
        reset() { this.state.x = 0; },
        step(dt, p) { this.state.x += p.v * dt; },
        draw(ctx, w, h, st) {
            ctx.fillStyle = '#ffffff'; ctx.fillRect(0, 0, w, h);
            ctx.save(); ctx.beginPath(); ctx.rect(0, 0, w, h); ctx.clip();
            ctx.fillStyle = 'rgba(37, 99, 235, 0.9)';
            ctx.beginPath(); ctx.arc(100 + st.x, 200, 20, 0, Math.PI * 2); ctx.fill();
            drawVector(ctx, 100 + st.x, 200, 40, 0, '#ef4444', 'v');
            ctx.font = 'bold 14px Inter'; ctx.fillText('x = ' + st.x.toFixed(1), 20, 30);
            ctx.restore();
        },
    };
    return {
        id: config.id, title: 'Fixture lesson', presets: [ball],
        introTimeline: [{ text: 'A ball rolls along a line.', preset: 'ball', audioId: 1, pauseAfterMs: 200,
            action: () => animateSlider('v', 0, 50, 500) }],
        lectureTimeline: [],
        derivations: { ball: [{ title: 'Kinematics', math: 'x = vt', desc: 'constant speed' }] },
        updateMathOverlay,
    };
}
