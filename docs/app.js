/**
 * VRChat Phone Link - Camera Streaming Application
 * Sends camera frames to desktop app via Cloudflare Tunnel
 */

// Configuration
const CONFIG = {
    targetFPS: 30,
    quality: {
        hd: { width: 1280, height: 720, jpeg: 0.8 },
        sd: { width: 640, height: 480, jpeg: 0.6 }
    },
    retryDelay: 3000,
    maxRetries: 5
};

// State
let state = {
    streaming: false,
    tunnelURL: null,
    facingMode: 'user',
    qualityMode: 'hd',
    stream: null,
    frameCount: 0,
    lastFrameTime: 0,
    currentFPS: 0,
    retryCount: 0
};

// DOM Elements
const elements = {
    video: document.getElementById('camera'),
    canvas: document.getElementById('canvas'),
    statusText: document.getElementById('status-text'),
    connectionDot: document.getElementById('connection-dot'),
    fpsCounter: document.getElementById('fps-counter'),
    btnFlip: document.getElementById('btn-flip'),
    btnQuality: document.getElementById('btn-quality'),
    qualityLabel: document.getElementById('quality-label'),
    errorMessage: document.getElementById('error-message'),
    errorText: document.getElementById('error-text'),
    btnRetry: document.getElementById('btn-retry')
};

// Initialize
async function init() {
    // Parse tunnel URL from query parameter
    const urlParams = new URLSearchParams(window.location.search);
    state.tunnelURL = urlParams.get('tunnel');

    if (!state.tunnelURL) {
        showError('No tunnel URL provided. Please scan the QR code from the desktop app.');
        return;
    }

    updateStatus('Requesting camera access...');

    try {
        await startCamera();
        updateStatus('Camera ready');
        startStreaming();
    } catch (error) {
        showError(`Camera error: ${error.message}`);
    }

    // Setup event listeners
    elements.btnFlip.addEventListener('click', toggleCamera);
    elements.btnQuality.addEventListener('click', toggleQuality);
    elements.btnRetry.addEventListener('click', () => {
        hideError();
        init();
    });
}

// Start camera with current settings
async function startCamera() {
    const quality = CONFIG.quality[state.qualityMode];

    try {
        // Stop existing stream if any
        if (state.stream) {
            state.stream.getTracks().forEach(track => track.stop());
        }

        state.stream = await navigator.mediaDevices.getUserMedia({
            video: {
                facingMode: state.facingMode,
                width: { ideal: quality.width },
                height: { ideal: quality.height },
                frameRate: { ideal: CONFIG.targetFPS }
            },
            audio: false
        });

        elements.video.srcObject = state.stream;

        // Mirror front camera
        if (state.facingMode === 'user') {
            elements.video.style.transform = 'scaleX(-1)';
        } else {
            elements.video.style.transform = 'scaleX(1)';
        }
    } catch (error) {
        console.error('Camera error:', error);
        throw error;
    }
}

// Start streaming frames
function startStreaming() {
    state.streaming = true;
    state.retryCount = 0;
    updateStatus('Connecting to desktop...');
    requestAnimationFrame(sendFrame);
}

// Send single frame
async function sendFrame(timestamp) {
    if (!state.streaming) return;

    const ctx = elements.canvas.getContext('2d');
    const quality = CONFIG.quality[state.qualityMode];
    const frameInterval = 1000 / CONFIG.targetFPS;

    // Throttle to target FPS
    if (timestamp - state.lastFrameTime >= frameInterval) {
        if (elements.video.readyState === elements.video.HAVE_ENOUGH_DATA) {
            // Setup canvas
            elements.canvas.width = elements.video.videoWidth;
            elements.canvas.height = elements.video.videoHeight;

            // Draw frame (flip if front camera)
            ctx.save();
            if (state.facingMode === 'user') {
                ctx.translate(elements.canvas.width, 0);
                ctx.scale(-1, 1);
            }
            ctx.drawImage(elements.video, 0, 0);
            ctx.restore();

            // Convert to JPEG blob
            elements.canvas.toBlob(async (blob) => {
                if (blob) {
                    try {
                        const response = await fetch(`https://${state.tunnelURL}/video`, {
                            method: 'POST',
                            headers: { 'Content-Type': 'image/jpeg' },
                            body: blob
                        });

                        if (response.ok) {
                            // Success
                            markConnected();
                            state.retryCount = 0;

                            // Update FPS
                            state.frameCount++;
                            const elapsed = timestamp - state.lastFrameTime;
                            state.currentFPS = Math.round(1000 / elapsed);
                            elements.fpsCounter.textContent = `${state.currentFPS} FPS`;
                        } else {
                            throw new Error(`Server returned ${response.status}`);
                        }
                    } catch (error) {
                        console.error('Send error:', error);
                        handleConnectionError(error);
                    }
                }
            }, 'image/jpeg', quality.jpeg);

            state.lastFrameTime = timestamp;
        }
    }

    requestAnimationFrame(sendFrame);
}

// Toggle camera (front/back)
async function toggleCamera() {
    state.facingMode = state.facingMode === 'user' ? 'environment' : 'user';
    updateStatus('Switching camera...');

    try {
        await startCamera();
        updateStatus('Camera switched');
    } catch (error) {
        showError(`Failed to switch camera: ${error.message}`);
    }
}

// Toggle quality (HD/SD)
function toggleQuality() {
    state.qualityMode = state.qualityMode === 'hd' ? 'sd' : 'hd';
    elements.qualityLabel.textContent = state.qualityMode.toUpperCase();

    updateStatus('Changing quality...');
    startCamera().then(() => {
        updateStatus(`Quality: ${state.qualityMode.toUpperCase()}`);
    });
}

// Connection status
function markConnected() {
    elements.connectionDot.classList.add('connected');
    if (elements.statusText.textContent === 'Connecting to desktop...') {
        updateStatus('LIVE');
    }
}

function markDisconnected() {
    elements.connectionDot.classList.remove('connected');
    updateStatus('Disconnected');
}

function updateStatus(text) {
    elements.statusText.textContent = text;
}

function handleConnectionError(error) {
    state.retryCount++;

    if (state.retryCount >= CONFIG.maxRetries) {
        state.streaming = false;
        markDisconnected();
        showError(`Connection lost: ${error.message}\n\nPlease check that the desktop app is still running.`);
    } else {
        markDisconnected();
        updateStatus(`Retrying (${state.retryCount}/${CONFIG.maxRetries})...`);
    }
}

// Error handling
function showError(message) {
    elements.errorText.textContent = message;
    elements.errorMessage.classList.remove('hidden');
}

function hideError() {
    elements.errorMessage.classList.add('hidden');
}

// Start when page loads
window.addEventListener('load', init);

// Prevent screen sleep
if ('wakeLock' in navigator) {
    let wakeLock = null;

    async function requestWakeLock() {
        try {
            wakeLock = await navigator.wakeLock.request('screen');
        } catch (err) {
            console.warn('Wake Lock error:', err);
        }
    }

    requestWakeLock();

    document.addEventListener('visibilitychange', () => {
        if (wakeLock !== null && document.visibilityState === 'visible') {
            requestWakeLock();
        }
    });
}
