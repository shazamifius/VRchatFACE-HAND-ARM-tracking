const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ============================================================================
// State
// ============================================================================
let isTracking = false;
let cameras = [];

// ============================================================================
// DOM Elements
// ============================================================================
const elements = {
  statusIndicator: document.getElementById('status-indicator'),
  videoFrame: document.getElementById('video-frame'),
  landmarkCanvas: document.getElementById('landmark-canvas'),
  videoPlaceholder: document.getElementById('video-placeholder'),
  videoOverlay: document.getElementById('video-overlay'),
  fpsDisplay: document.getElementById('fps-display'),
  faceIndicator: document.getElementById('face-indicator'),
  lhandIndicator: document.getElementById('lhand-indicator'),
  rhandIndicator: document.getElementById('rhand-indicator'),
  cameraSelect: document.getElementById('camera-select'),
  refreshCamerasBtn: document.getElementById('refresh-cameras-btn'),
  cameraInfo: document.getElementById('camera-info'),
  toggleFace: document.getElementById('toggle-face'),
  toggleHands: document.getElementById('toggle-hands'),
  togglePose: document.getElementById('toggle-pose'),
  faceStatus: document.getElementById('face-status'),
  handsStatus: document.getElementById('hands-status'),
  poseStatus: document.getElementById('pose-status'),
  oscIp: document.getElementById('osc-ip'),
  oscPort: document.getElementById('osc-port'),
  oscStatus: document.getElementById('osc-status'),
  startBtn: document.getElementById('start-btn'),
  stopBtn: document.getElementById('stop-btn'),
  latencyValue: document.getElementById('latency-value'),
  calibNeutralBtn: document.getElementById('calib-neutral-btn'),
  calibTPoseBtn: document.getElementById('calib-tpose-btn'),
  phoneModeBtn: document.getElementById('phone-mode-btn'),
};

const ctx = elements.landmarkCanvas ? elements.landmarkCanvas.getContext('2d') : null;

// ============================================================================
// Initialize
// ============================================================================
document.addEventListener('DOMContentLoaded', async () => {
  console.log('VRChat Bridge Hub v2 initialized');
  setupEventListeners();
  await refreshCameras();
});

// Phone Connected Event
listen('phone-connected', (event) => {
  console.log("Phone Connected!");
  const modal = document.getElementById('qr-modal');
  if (modal && !modal.classList.contains('hidden')) {
    modal.classList.add('hidden');
    modal.classList.remove('active');
  }
  const statusInd = document.getElementById('status-indicator');
  if (statusInd) {
    statusInd.classList.add('connected');
    const text = statusInd.querySelector('.status-text');
    if (text) text.innerText = 'Phone Connected';
  }
});

function setupEventListeners() {
  elements.startBtn.addEventListener('click', startTracking);
  elements.stopBtn.addEventListener('click', stopTracking);
  elements.refreshCamerasBtn.addEventListener('click', refreshCameras);
  elements.cameraSelect.addEventListener('change', onCameraChange);
  elements.phoneModeBtn.addEventListener('click', setupPhoneMode);

  elements.toggleFace.addEventListener('change', updateModuleStatus);
  elements.toggleHands.addEventListener('change', updateModuleStatus);
  elements.togglePose.addEventListener('change', updateModuleStatus);

  elements.calibNeutralBtn.addEventListener('click', () => startCalibration('Neutral'));
  elements.calibTPoseBtn.addEventListener('click', () => startCalibration('TPose'));

  document.querySelectorAll('.segment-btn').forEach(btn => {
    btn.addEventListener('click', (e) => {
      document.querySelectorAll('.segment-btn').forEach(b => b.classList.remove('active'));
      e.target.classList.add('active');
      const quality = e.target.dataset.quality;

      const descendants = {
        'Low': "Smooth tracking, low CPU.",
        'Medium': "Balanced performance.",
        'High': "Fast response, higher jitter."
      };
      const desc = document.getElementById('quality-desc');
      if (desc) desc.innerText = descendants[quality] || "";

      invoke('set_tracking_quality', { quality: quality }).catch(console.error);
    });
  });

  elements.videoFrame.onerror = (e) => {
    // console.error("Video Frame Load Error", e);
    // Ignore for now, normal during startup
  };
}

// ============================================================================
// Camera Management
// ============================================================================
async function refreshCameras() {
  elements.cameraSelect.innerHTML = '<option value="-1">Detecting...</option>';
  elements.cameraInfo.textContent = 'Scanning for cameras...';

  const timeout = new Promise((_, reject) => {
    setTimeout(() => reject(new Error("Camera scan timeout")), 10000);
  });

  try {
    cameras = await Promise.race([
      invoke('get_cameras'),
      timeout
    ]);
    updateCameraList(cameras);
    elements.cameraInfo.textContent = `Found ${cameras.length} devices`;
  } catch (error) {
    console.error('Failed to get cameras:', error);
    elements.cameraInfo.textContent = 'Scan failed. Showing manual options.';
    cameras = [];
    updateCameraList([]);
  }
}

function updateCameraList(cams) {
  elements.cameraSelect.innerHTML = '';

  // Manual Fallbacks for safety
  const manualOptions = [
    { index: 0, name: "FORCE Camera Index 0 (Manual)" },
    { index: 1, name: "FORCE Camera Index 1 (Manual)" },
  ];

  manualOptions.forEach(opt => {
    const option = document.createElement('option');
    option.value = opt.index;
    option.text = opt.name;
    elements.cameraSelect.add(option);
  });

  // Divider
  const divider = document.createElement('option');
  divider.text = "--- Detected Devices ---";
  divider.disabled = true;
  elements.cameraSelect.add(divider);

  // Detected
  if (!cams || cams.length === 0) {
    const option = document.createElement('option');
    option.text = "No devices detected (Use Manual)";
    option.disabled = true;
    elements.cameraSelect.add(option);
  } else {
    cams.forEach(cam => {
      const option = document.createElement('option');
      option.value = cam.index;
      option.text = `${cam.name} (${cam.index})`;
      elements.cameraSelect.add(option);
    });
  }

  // Phone
  const phoneDivider = document.createElement('option');
  phoneDivider.text = "--- Remote ---";
  phoneDivider.disabled = true;
  elements.cameraSelect.add(phoneDivider);

  const phoneOption = document.createElement('option');
  phoneOption.value = 999;
  phoneOption.text = "📱 Phone Camera (Remote)";
  elements.cameraSelect.add(phoneOption);
}

function onCameraChange() {
  const val = elements.cameraSelect.value;
  if (val == 999) {
    elements.cameraInfo.textContent = "Remote Stream • MJPEG/Push";
    return;
  }
  const selected = cameras.find(c => c.index == val);
  if (selected) {
    elements.cameraInfo.textContent = `${selected.name} • ${selected.backend || 'Auto'}`;
  } else {
    elements.cameraInfo.textContent = "Manual Selection";
  }
}

// ============================================================================
// Tracking Control
// ============================================================================
async function startTracking() {
  const cameraIndex = parseInt(elements.cameraSelect.value);
  const oscIp = elements.oscIp.value;
  const oscPort = parseInt(elements.oscPort.value);

  setUILoading(true);

  try {
    // Collect active segment for visual feedback if needed, but backend handles config
    // We didn't add resolution params to UI yet, so passing defaults via backend logic (handled by backend or we can add args)
    // Updated lib.rs start_tracking accepts width/height/fps. Let's pass nulls to use defaults for now.

    await invoke('start_tracking', {
      cameraIndex,
      oscIp,
      oscPort,
      width: 640,
      height: 480,
      fps: 30,
      format: null
    });

    isTracking = true;
    updateUIState();
    startVideoStream();

  } catch (error) {
    console.error('Failed to start tracking:', error);
    alert(`Failed to start tracking:\n${error}`);
    setUILoading(false);
  }
}

async function stopTracking() {
  try {
    await invoke('stop_tracking');
  } catch (e) { console.error(e); }

  isTracking = false;
  stopVideoStream();
  updateUIState();
  setUILoading(false);
}

// ============================================================================
// Landmark Visualization (No raw video — just skeleton + stats)
// ============================================================================
let renderInterval = null;

// MediaPipe Face Mesh tessellation edges (subset for wireframe)
const FACE_CONNECTIONS = [
  // Jawline
  [10, 338], [338, 297], [297, 332], [332, 284], [284, 251], [251, 389], [389, 356], [356, 454],
  [454, 323], [323, 361], [361, 288], [288, 397], [397, 365], [365, 379], [379, 378], [378, 400],
  [400, 377], [377, 152], [152, 148], [148, 176], [176, 149], [149, 150], [150, 136], [136, 172],
  [172, 58], [58, 132], [132, 93], [93, 234], [234, 127], [127, 162], [162, 21], [21, 54],
  [54, 103], [103, 67], [67, 109], [109, 10],
  // Left eye
  [33, 7], [7, 163], [163, 144], [144, 145], [145, 153], [153, 154], [154, 155], [155, 133],
  [133, 173], [173, 157], [157, 158], [158, 159], [159, 160], [160, 161], [161, 246], [246, 33],
  // Right eye
  [362, 382], [382, 381], [381, 380], [380, 374], [374, 373], [373, 390], [390, 249], [249, 263],
  [263, 466], [466, 388], [388, 387], [387, 386], [386, 385], [385, 384], [384, 398], [398, 362],
  // Lips outer
  [61, 146], [146, 91], [91, 181], [181, 84], [84, 17], [17, 314], [314, 405], [405, 321],
  [321, 375], [375, 291], [291, 409], [409, 270], [270, 269], [269, 267], [267, 0], [0, 37],
  [37, 39], [39, 40], [40, 185], [185, 61],
  // Nose
  [168, 6], [6, 197], [197, 195], [195, 5], [5, 4], [4, 1], [1, 19], [19, 94], [94, 2],
];

// Hand connections (MediaPipe 21-point model)
const HAND_CONNECTIONS = [
  [0, 1], [1, 2], [2, 3], [3, 4],       // Thumb
  [0, 5], [5, 6], [6, 7], [7, 8],       // Index
  [0, 9], [9, 10], [10, 11], [11, 12],  // Middle
  [0, 13], [13, 14], [14, 15], [15, 16],// Ring
  [0, 17], [17, 18], [18, 19], [19, 20],// Pinky
  [5, 9], [9, 13], [13, 17],          // Palm
];

async function startVideoStream() {
  elements.videoPlaceholder.classList.add('hidden');
  if (elements.landmarkCanvas) elements.landmarkCanvas.classList.add('active');
  elements.videoOverlay.classList.add('active');

  if (renderInterval) clearInterval(renderInterval);
  renderInterval = setInterval(renderLoop, 50); // ~20 FPS for visualization
}

function stopVideoStream() {
  if (renderInterval) clearInterval(renderInterval);
  renderInterval = null;

  // Clear canvas
  if (ctx && elements.landmarkCanvas) {
    ctx.clearRect(0, 0, elements.landmarkCanvas.width, elements.landmarkCanvas.height);
  }
  if (elements.landmarkCanvas) elements.landmarkCanvas.classList.remove('active');
  elements.videoPlaceholder.classList.remove('hidden');
  elements.videoOverlay.classList.remove('active');
}

async function renderLoop() {
  if (!isTracking) return;

  try {
    // Fetch status + tracking data in parallel
    const [status, trackingData] = await Promise.all([
      invoke('get_tracking_status'),
      invoke('get_tracking_data'),
    ]);

    // === Update Status UI ===
    if (status) {
      elements.fpsDisplay.textContent = Math.round(status.fps || 0);

      const statusInd = document.getElementById('status-indicator');
      if (status.running) {
        statusInd.classList.add('connected');
        statusInd.querySelector('.status-text').innerText = "Running";
      }

      const qualityStatus = document.getElementById('quality-status');
      if (qualityStatus) {
        const span = qualityStatus.querySelector('span:last-child');
        const dot = qualityStatus.querySelector('.osc-dot');
        if (span && dot) {
          if (status.fps > 25) {
            span.textContent = "Good"; dot.style.background = "var(--success)";
          } else if (status.fps > 10) {
            span.textContent = "Weak"; dot.style.background = "var(--warning)";
          } else {
            span.textContent = "Poor"; dot.style.background = "var(--danger)";
          }
        }
      }

      checkPhoneConnection(status);
      updateIndicator(elements.faceIndicator, status.face_detected);
      updateIndicator(elements.lhandIndicator, status.left_hand_detected);
      updateIndicator(elements.rhandIndicator, status.right_hand_detected);
    }

    // === Draw Landmarks on Canvas ===
    if (ctx && elements.landmarkCanvas) {
      const W = elements.landmarkCanvas.width;
      const H = elements.landmarkCanvas.height;

      // Clear with dark background
      ctx.fillStyle = '#0a0a0f';
      ctx.fillRect(0, 0, W, H);

      // Draw grid (subtle)
      ctx.strokeStyle = 'rgba(255,255,255,0.04)';
      ctx.lineWidth = 1;
      for (let x = 0; x < W; x += 40) {
        ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, H); ctx.stroke();
      }
      for (let y = 0; y < H; y += 40) {
        ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(W, y); ctx.stroke();
      }

      // Draw face landmarks
      if (trackingData && trackingData.face_landmarks) {
        const pts = trackingData.face_landmarks;
        // Scale landmarks from camera coords to canvas
        // Landmarks are in pixel coords of the camera frame
        const camW = status ? (status.fps > 0 ? 1920 : 640) : 640; // Approximate
        const camH = status ? (status.fps > 0 ? 1080 : 480) : 480;
        const scaleX = W / camW;
        const scaleY = H / camH;

        // Draw wireframe connections
        ctx.strokeStyle = 'rgba(0, 255, 136, 0.5)';
        ctx.lineWidth = 1;
        for (const [a, b] of FACE_CONNECTIONS) {
          if (a < pts.length && b < pts.length) {
            ctx.beginPath();
            ctx.moveTo(pts[a][0] * scaleX, pts[a][1] * scaleY);
            ctx.lineTo(pts[b][0] * scaleX, pts[b][1] * scaleY);
            ctx.stroke();
          }
        }

        // Draw points
        ctx.fillStyle = '#00ff88';
        for (let i = 0; i < pts.length; i++) {
          const x = pts[i][0] * scaleX;
          const y = pts[i][1] * scaleY;
          ctx.fillRect(x - 1, y - 1, 2, 2);
        }
      }

      // Draw left hand
      if (trackingData && trackingData.left_hand_landmarks) {
        drawHand(trackingData.left_hand_landmarks, '#ff6b35', 'L');
      }

      // Draw right hand
      if (trackingData && trackingData.right_hand_landmarks) {
        drawHand(trackingData.right_hand_landmarks, '#4ecdc4', 'R');
      }

      // Draw stats text
      ctx.fillStyle = 'rgba(255,255,255,0.7)';
      ctx.font = '12px Inter, sans-serif';
      ctx.fillText(`FPS: ${Math.round(status?.fps || 0)}`, 10, 20);
      ctx.fillText(`Frame: ${(status?.frame_time_ms || 0).toFixed(1)}ms`, 10, 36);
      if (trackingData?.face_landmarks) {
        ctx.fillText(`Face: ${trackingData.face_landmarks.length} pts`, 10, 52);
      }
      if (!trackingData?.face_landmarks && !trackingData?.left_hand_landmarks && !trackingData?.right_hand_landmarks) {
        ctx.fillStyle = 'rgba(255,255,255,0.3)';
        ctx.font = '16px Inter, sans-serif';
        ctx.textAlign = 'center';
        ctx.fillText('Waiting for landmarks...', W / 2, H / 2);
        ctx.textAlign = 'left';
      }
    }
  } catch (e) {
    console.error(e);
  }
}

function drawHand(pts, color, label) {
  if (!ctx || !elements.landmarkCanvas) return;
  const W = elements.landmarkCanvas.width;
  const H = elements.landmarkCanvas.height;
  const camW = 640, camH = 480;
  const scaleX = W / camW;
  const scaleY = H / camH;

  // Connections
  ctx.strokeStyle = color;
  ctx.lineWidth = 2;
  for (const [a, b] of HAND_CONNECTIONS) {
    if (a < pts.length && b < pts.length) {
      ctx.beginPath();
      ctx.moveTo(pts[a][0] * scaleX, pts[a][1] * scaleY);
      ctx.lineTo(pts[b][0] * scaleX, pts[b][1] * scaleY);
      ctx.stroke();
    }
  }

  // Points
  ctx.fillStyle = color;
  for (const pt of pts) {
    ctx.beginPath();
    ctx.arc(pt[0] * scaleX, pt[1] * scaleY, 3, 0, Math.PI * 2);
    ctx.fill();
  }

  // Label
  if (pts.length > 0) {
    ctx.fillStyle = color;
    ctx.font = 'bold 14px Inter, sans-serif';
    ctx.fillText(label, pts[0][0] * scaleX - 10, pts[0][1] * scaleY - 10);
  }
}

function updateIndicator(el, active) {
  if (active) el.classList.add('active');
  else el.classList.remove('active');
}


// ============================================================================
// UI & Helpers
// ============================================================================
function updateUIState() {
  if (isTracking) {
    elements.startBtn.classList.add('hidden');
    elements.stopBtn.classList.remove('hidden');
    elements.oscStatus.querySelector('span:last-child').innerText = "Connected";
    elements.oscStatus.querySelector('.osc-dot').style.background = "var(--success)";
  } else {
    elements.startBtn.classList.remove('hidden');
    elements.stopBtn.classList.add('hidden');
    elements.oscStatus.querySelector('span:last-child').innerText = "Disconnected";
    elements.oscStatus.querySelector('.osc-dot').style.background = "#666";

    // Reset indicators
    elements.faceIndicator.classList.remove('active');
    elements.lhandIndicator.classList.remove('active');
    elements.rhandIndicator.classList.remove('active');
    elements.fpsDisplay.innerText = "0";
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
    elements.startBtn.innerText = "Processing...";
  } else {
    elements.startBtn.innerHTML = `
      <svg viewBox="0 0 24 24" fill="none"><polygon points="5,3 19,12 5,21" fill="currentColor"/></svg>
      <span>Start Tracking</span>`;
  }
}

function checkPhoneConnection(status) {
  const modal = document.getElementById('qr-modal');
  if (status.phone_connected && modal && !modal.classList.contains('hidden')) {
    modal.classList.add('hidden');
    modal.classList.remove('active');
  }
}

// ============================================================================
// Calibration
// ============================================================================
async function startCalibration(mode) {
  if (!isTracking) { alert("Start tracking first!"); return; }
  const btn = mode === 'Neutral' ? elements.calibNeutralBtn : elements.calibTPoseBtn;
  const original = btn.innerText;

  try {
    await invoke('start_calibration', { mode });
    btn.innerText = "⏳ Calibrating...";
    btn.disabled = true;
    setTimeout(() => {
      btn.innerText = "✅ Done!";
      setTimeout(() => { btn.innerText = original; btn.disabled = false; }, 1500);
    }, 2000);
  } catch (e) {
    alert("Calibration failed: " + e);
    btn.innerText = original;
    btn.disabled = false;
  }
}

// ============================================================================
// Phone Mode
// ============================================================================
async function setupPhoneMode() {
  const btn = elements.phoneModeBtn;
  const original = btn.innerText;
  btn.innerText = "Loading...";
  btn.disabled = true;

  const container = document.getElementById('qr-container');
  if (container) container.innerHTML = "Generating...";

  const modal = document.getElementById('qr-modal');

  try {
    const localIp = await invoke('get_local_ip');
    const url = `http://${localIp}:9001/`;

    let qrSvg = "";
    try {
      qrSvg = await invoke('generate_qr_code', { data: url });
    } catch (e) {
      console.error(e);
      qrSvg = "<p>QR Gen Failed</p>";
    }

    if (container) {
      container.innerHTML = `
              ${qrSvg}
              <p style="margin-top:10px;">Network: <b>${url}</b></p>
              <button id="use-cloud-btn" class="btn-outline-small" style="width:100%; margin-top:10px;">Use Cloudflare</button>
            `;

      document.getElementById('use-cloud-btn').onclick = () => {
        setupCloudflared(container);
      };
    }

    if (modal) {
      modal.classList.remove('hidden');
      modal.classList.add('active');
      const close = modal.querySelector('.close-modal');
      if (close) close.onclick = () => { modal.classList.remove('active'); modal.classList.add('hidden'); };
    }

  } catch (e) {
    alert(e);
  } finally {
    btn.innerText = original;
    btn.disabled = false;
  }
}

async function setupCloudflared(container) {
  container.innerHTML = "Starting Tunnel...";
  try {
    await invoke('start_cloudflare_tunnel', { port: 9001 });
    // Poll for QR
    let qrSvg = null;
    for (let i = 0; i < 20; i++) {
      await new Promise(r => setTimeout(r, 1000));
      qrSvg = await invoke('get_tunnel_qr');
      if (qrSvg) break;
    }

    if (qrSvg) {
      container.innerHTML = `${qrSvg}<p>Cloudflare Active</p>`;
    }
  } catch (e) {
    container.innerHTML = "Failed: " + e;
  }
}
