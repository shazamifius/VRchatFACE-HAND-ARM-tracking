const { invoke } = window.__TAURI__.core;

// ============================================================================
// State
// ============================================================================
let isTracking = false;
let cameras = [];
let statusUpdateInterval = null;

// ============================================================================
// DOM Elements
// ============================================================================
const elements = {
  // Status
  statusIndicator: document.getElementById('status-indicator'),

  // Video
  videoFrame: document.getElementById('video-frame'),
  videoPlaceholder: document.getElementById('video-placeholder'),
  videoOverlay: document.getElementById('video-overlay'),
  fpsDisplay: document.getElementById('fps-display'),

  // Tracking indicators
  faceIndicator: document.getElementById('face-indicator'),
  lhandIndicator: document.getElementById('lhand-indicator'),
  rhandIndicator: document.getElementById('rhand-indicator'),

  // Camera
  cameraSelect: document.getElementById('camera-select'),
  refreshCamerasBtn: document.getElementById('refresh-cameras-btn'),
  cameraInfo: document.getElementById('camera-info'),

  // Toggles
  toggleFace: document.getElementById('toggle-face'),
  toggleHands: document.getElementById('toggle-hands'),
  togglePose: document.getElementById('toggle-pose'),
  faceStatus: document.getElementById('face-status'),
  handsStatus: document.getElementById('hands-status'),
  poseStatus: document.getElementById('pose-status'),

  // OSC
  oscIp: document.getElementById('osc-ip'),
  oscPort: document.getElementById('osc-port'),
  oscStatus: document.getElementById('osc-status'),

  // Actions
  startBtn: document.getElementById('start-btn'),
  stopBtn: document.getElementById('stop-btn'),
  latencyValue: document.getElementById('latency-value'),

  // Phone mode
  phoneModeBtn: document.getElementById('phone-mode-btn'),
};

// ============================================================================
// Initialize
// ============================================================================
document.addEventListener('DOMContentLoaded', async () => {
  console.log('VRChat Bridge Hub v2 initialized');

  // Setup event listeners
  setupEventListeners();

  // Load cameras
  await refreshCameras();
});

function setupEventListeners() {
  elements.startBtn.addEventListener('click', startTracking);
  elements.stopBtn.addEventListener('click', stopTracking);
  elements.refreshCamerasBtn.addEventListener('click', refreshCameras);
  elements.cameraSelect.addEventListener('change', onCameraChange);
  elements.phoneModeBtn.addEventListener('click', setupPhoneMode);

  // Toggle listeners
  elements.toggleFace.addEventListener('change', updateModuleStatus);
  elements.toggleHands.addEventListener('change', updateModuleStatus);
  elements.togglePose.addEventListener('change', updateModuleStatus);

  // Video Error Logging
  elements.videoFrame.onerror = (e) => {
    console.error("Video Frame Load Error:", elements.videoFrame.src, e);
    // Retry with 127.0.0.1 if localhost fails
    if (elements.videoFrame.src.includes("localhost")) {
      console.log("Retrying with 127.0.0.1...");
      // This logic is handled in the interval, but it's good to know.
    }
  };
}

// ============================================================================
// Camera Management
// ============================================================================
async function refreshCameras() {
  elements.cameraSelect.innerHTML = '<option value="-1">Detecting...</option>';
  elements.cameraInfo.textContent = 'Scanning for cameras...';

  try {
    cameras = await invoke('get_cameras');
    updateCameraList(cameras);
  } catch (error) {
    console.error('Failed to get cameras:', error);
    cameras = [];
  }
  updateCameraList(cameras);
}

function updateCameraList(cameraList) {
  elements.cameraSelect.innerHTML = '';

  if (!cameraList || cameraList.length === 0) {
    // Even if no local cameras, allow Phone Camera
    elements.cameraSelect.innerHTML = '';
  }

  // Add Phone Camera Option
  const phoneOption = document.createElement('option');
  phoneOption.value = 999;
  phoneOption.textContent = "📱 Phone Camera (Remote)";
  elements.cameraSelect.appendChild(phoneOption);

  if (!cameraList || cameraList.length === 0) {
    elements.cameraInfo.textContent = 'No local cameras. Use Phone Camera.';
    return;
  }

  cameraList.forEach((cam, i) => {
    const option = document.createElement('option');
    option.value = cam.index !== undefined ? cam.index : i;
    option.textContent = cam.name || `Camera ${i}`;
    elements.cameraSelect.appendChild(option);
  });

  const selected = cameraList[0];
  elements.cameraInfo.textContent = `${selected.resolution ? `${selected.resolution[0]}x${selected.resolution[1]}` : 'Ready'} • ${selected.backend || 'DirectShow'}`;
}

function onCameraChange() {
  const val = elements.cameraSelect.value;
  if (val == 999) {
    elements.cameraInfo.textContent = "Remote Stream • MJPEG/Push";
    return;
  }

  const selected = cameras.find(c => c.index == val);
  if (selected) {
    elements.cameraInfo.textContent = `${selected.resolution ? `${selected.resolution[0]}x${selected.resolution[1]}` : 'Ready'} • ${selected.backend || 'DirectShow'}`;
  }
}

// ============================================================================
// Tracking Control
// ============================================================================
async function startTracking() {
  const cameraIndex = parseInt(elements.cameraSelect.value);
  const oscIp = elements.oscIp.value;
  const oscPort = parseInt(elements.oscPort.value);

  if (cameraIndex < 0) {
    alert('Please select a valid camera');
    return;
  }

  setUILoading(true);

  try {
    const result = await invoke('start_tracking', {
      cameraIndex,
      oscIp,
      oscPort
    });

    if (result) {
      isTracking = true;
      updateUIState();
      startVideoStream();
    }
  } catch (error) {
    console.error('Failed to start tracking:', error);
    alert(`Failed to start tracking:\n${error}`);
  }

  setUILoading(false);
}

async function stopTracking() {
  try {
    await invoke('stop_tracking');
  } catch (error) {
    console.error('Failed to stop tracking:', error);
  }

  isTracking = false;
  stopVideoStream();
  updateUIState();
}

// ============================================================================
// Video Streaming
// ============================================================================
// ============================================================================
// Video Streaming (Snapshot Fallback)
// ============================================================================
let videoInterval = null;

function startVideoStream() {
  // Show video, hide placeholder
  elements.videoPlaceholder.classList.add('hidden');
  elements.videoFrame.classList.add('active');
  elements.videoOverlay.classList.add('active');

  // [MODIFIED] Use Python Tracker MJPEG Stream directly
  // This provides high FPS low latency preview without complex fetching
  const streamUrl = "http://localhost:8080/";

  // Force reload image by appending timestamp to avoid caching/stale state
  elements.videoFrame.src = `${streamUrl}?t=${Date.now()}`;

  elements.videoFrame.onerror = () => {
    console.warn("MJPEG Stream lost. Retrying...");
    setTimeout(() => {
      if (isTracking) {
        elements.videoFrame.src = `${streamUrl}?t=${Date.now()}`;
      }
    }, 1000);
  };

  // Start polling for status updates only
  if (statusUpdateInterval) clearInterval(statusUpdateInterval);
  statusUpdateInterval = setInterval(updateTrackingStatus, 100);
}

function stopVideoStream() {
  if (statusUpdateInterval) {
    clearInterval(statusUpdateInterval);
    statusUpdateInterval = null;
  }

  if (videoInterval) {
    clearInterval(videoInterval);
    videoInterval = null;
  }

  // Stop video stream
  elements.videoFrame.src = "";
  elements.videoFrame.removeAttribute('src');

  // Hide video, show placeholder
  elements.videoFrame.classList.remove('active');
  elements.videoPlaceholder.classList.remove('hidden');
  elements.videoOverlay.classList.remove('active');
}

async function updateTrackingStatus() {
  if (!isTracking) return;

  try {
    const status = await invoke('get_tracking_status');

    if (status) {
      // Update FPS
      elements.fpsDisplay.textContent = Math.round(status.fps || 0);

      checkPhoneConnection(status); // [NEW]
      // Update latency
      elements.latencyValue.textContent = Math.round(status.frame_time_ms || 0);

      // Update tracking indicators
      updateIndicator(elements.faceIndicator, status.face_detected);
      updateIndicator(elements.lhandIndicator, status.left_hand_detected);
      updateIndicator(elements.rhandIndicator, status.right_hand_detected);

      // Update module status badges
      updateModuleStatus();
    }
  } catch (error) {
    // Silent fail for status updates
  }
}

function updateIndicator(element, isActive) {
  if (isActive) {
    element.classList.add('active');
  } else {
    element.classList.remove('active');
  }
}

// ============================================================================
// UI State Management
// ============================================================================
function updateUIState() {
  if (isTracking) {
    elements.startBtn.classList.add('hidden');
    elements.stopBtn.classList.remove('hidden');
    // The status indicator logic is now handled by updateTrackingStatus
    elements.oscStatus.classList.add('connected');
    elements.oscStatus.querySelector('span:last-child').textContent = 'Connected';
  } else {
    elements.startBtn.classList.remove('hidden');
    elements.stopBtn.classList.add('hidden');
    elements.statusIndicator.classList.remove('connected');
    elements.statusIndicator.querySelector('.status-text').textContent = 'Disconnected';
    elements.oscStatus.classList.remove('connected');
    elements.oscStatus.querySelector('span:last-child').textContent = 'Disconnected';

    // Reset indicators
    elements.faceIndicator.classList.remove('active');
    elements.lhandIndicator.classList.remove('active');
    elements.rhandIndicator.classList.remove('active');
    elements.fpsDisplay.textContent = '0';
    elements.latencyValue.textContent = '--';
  }
}

function updateModuleStatus() {
  elements.faceStatus.textContent = elements.toggleFace.checked ? 'On' : 'Off';
  elements.faceStatus.classList.toggle('active', elements.toggleFace.checked);

  elements.handsStatus.textContent = elements.toggleHands.checked ? 'On' : 'Off';
  elements.handsStatus.classList.toggle('active', elements.toggleHands.checked);

  elements.poseStatus.textContent = elements.togglePose.checked ? 'On' : 'Off';
  elements.poseStatus.classList.toggle('active', elements.togglePose.checked);
}

function setUILoading(loading) {
  elements.startBtn.disabled = loading;
  elements.stopBtn.disabled = loading;
  if (loading) {
    elements.startBtn.innerHTML = '<span>Starting...</span>';
  } else {
    elements.startBtn.innerHTML = `
      <svg viewBox="0 0 24 24" fill="none">
        <polygon points="5,3 19,12 5,21" fill="currentColor"/>
      </svg>
      <span>Start Tracking</span>
    `;
  }
}

// ============================================================================
async function setupPhoneMode() {
  const btn = elements.phoneModeBtn;
  const originalText = btn.textContent;

  console.log("Setting up Phone Mode...");
  btn.textContent = "Loading...";
  btn.disabled = true;

  try {
    let qrSvg = await invoke('get_tunnel_qr');
    console.log("Initial QR SVG:", qrSvg ? "Found" : "Not Found");

    if (!qrSvg) {
      console.log("Starting Cloudflare Tunnel...");
      const started = await invoke('start_cloudflare_tunnel', { port: 9001 });
      console.log("Tunnel Start Result:", started);

      if (started) {
        let attempts = 0;
        while (!qrSvg && attempts < 20) { // Increased attempts
          console.log(`Polling for QR... Attempt ${attempts + 1}`);
          await new Promise(r => setTimeout(r, 1000));
          qrSvg = await invoke('get_tunnel_qr');
          attempts++;
        }
      }
    }

    if (qrSvg) {
      console.log("Displaying QR Code");
      const modal = document.getElementById('qr-modal');
      const container = document.getElementById('qr-container');
      const closeBtn = document.querySelector('.close-modal');

      if (!modal || !container) {
        console.error("Critical: Modal elements not found in DOM");
        alert("Error: UI elements missing. Please restart app.");
        return;
      }

      container.innerHTML = qrSvg;

      // [FIX] Must remove hidden class because it has !important
      modal.classList.remove('hidden');
      modal.classList.add('active');

      const closeModal = () => {
        modal.classList.remove('active');
        modal.classList.add('hidden');
      };

      closeBtn.onclick = closeModal;

      window.onclick = function (event) {
        if (event.target == modal) {
          closeModal();
        }
      }
    } else {
      console.error("Timed out waiting for QR code");
      alert("Failed to generate Cloudflare Tunnel URL. Check console for details.");
    }

  } catch (error) {
    console.error("Setup Error:", error);
    alert('Phone mode error: ' + error);
  } finally {
    btn.textContent = originalText;
    btn.disabled = false;
  }
}

// Check for phone connection to auto-close modal
function checkPhoneConnection(status) {
  const modal = document.getElementById('qr-modal');
  if (status.phone_connected && modal && modal.classList.contains('active')) {
    console.log("Phone connected! Closing QR modal.");
    modal.classList.remove('active');
    modal.classList.add('hidden');
  }
}
