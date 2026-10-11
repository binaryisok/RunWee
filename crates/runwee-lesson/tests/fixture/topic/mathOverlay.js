import { showMathLine } from '../core/renderer.js';
export function updateMathOverlay(step, wi) {
    if (step === 1 && wi >= 0) showMathLine('Ramp', 'title');
    if (step === 1 && wi >= 3) showMathLine('x = vt', 'math', true);
}
