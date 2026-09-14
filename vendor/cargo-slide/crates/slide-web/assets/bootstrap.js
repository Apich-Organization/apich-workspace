import init from './slide_web.js';

init().then(() => {
    const spinner = document.getElementById('loading-spinner');
    if (spinner) {
        spinner.remove();
    }
}).catch((err) => {
    console.error('Failed to initialize Cargo Slide WASM:', err);
    const spinner = document.getElementById('loading-spinner');
    if (spinner) {
        spinner.innerHTML = '<p style="color: #ef4444;">Failed to load presentation. Check console for details.</p>';
    }
});
