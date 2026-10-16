const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let livingOrb = null;
try {
  const canvas = document.getElementById('byte-orb-canvas');
  const OrbClass = window.ByteLivingOrb || (typeof ByteLivingOrb !== 'undefined' ? ByteLivingOrb : null);
  if (OrbClass && canvas) {
    livingOrb = new OrbClass(canvas);
  } else {
    console.error('ByteLivingOrb class not found!');
  }
} catch (err) {
  console.error('Failed to initialize ByteLivingOrb:', err);
}

const statusText = document.getElementById('status-text');
const statusBadge = document.querySelector('.status-badge');
const presenceWidget = document.querySelector('.presence-widget');
const talkButton = document.getElementById('talk-button');

let isRunning = false;
let statusResetTimer = null;

function updateStatusBadge(state) {
  if (state !== 'error' && statusResetTimer) {
    clearTimeout(statusResetTimer);
    statusResetTimer = null;
  }
  statusBadge.className = `status-badge ${state}`;
  switch (state) {
    case 'idle':
      statusText.innerText = 'Click to talk · Ctrl+B';
      talkButton.setAttribute('aria-label', 'Start talking to Byte');
      talkButton.title = 'Click to talk · Ctrl+B';
      break;
    case 'listening':
      statusText.innerText = 'Listening · click to stop';
      talkButton.setAttribute('aria-label', 'Stop listening to Byte');
      talkButton.title = 'Click to stop';
      break;
    case 'thinking':
      statusText.innerText = 'Thinking · click to cancel';
      talkButton.setAttribute('aria-label', 'Cancel Byte’s response');
      talkButton.title = 'Click to cancel';
      break;
    case 'speaking':
      statusText.innerText = 'Speaking · click to stop';
      talkButton.setAttribute('aria-label', 'Stop Byte speaking');
      talkButton.title = 'Click to stop';
      break;
    case 'cancelled':
      statusText.innerText = 'Cancelled';
      talkButton.setAttribute('aria-label', 'Start talking to Byte');
      talkButton.title = 'Click to talk · Ctrl+B';
      break;
    case 'error':
      statusText.innerText = 'Couldn’t complete · click to retry';
      talkButton.setAttribute('aria-label', 'Retry talking to Byte');
      talkButton.title = 'Click to try again';
      break;
    case 'hidden':
      presenceWidget.classList.add('hidden');
      return;
  }
  presenceWidget.classList.remove('hidden');
}

// Full conversational lifecycle run
async function runInteraction() {
  if (isRunning) return;
  if (statusResetTimer) {
    clearTimeout(statusResetTimer);
    statusResetTimer = null;
  }
  isRunning = true;

  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    const win = getCurrentWindow();
    await win.show();
    await win.setFocus();
  } catch (err) {
    console.warn('Window focus/show notice:', err);
  }

  let result = null;
  let failed = false;
  try {
    result = await invoke('start_interaction');
    console.log('Interaction turn completed:', result);

    if (result && result.auto_listen) {
      setTimeout(() => {
        isRunning = false;
        runInteraction();
      }, 400);
      return;
    }
  } catch (err) {
    console.error('Interaction loop error:', err);
    failed = true;
  } finally {
    isRunning = false;
    if (livingOrb) livingOrb.setAudioLevel(0.0);
    if (!result || !result.auto_listen) {
      if (failed) {
        if (livingOrb) livingOrb.setState('idle');
        updateStatusBadge('error');
        statusResetTimer = setTimeout(() => updateStatusBadge('idle'), 3000);
      } else {
        if (livingOrb) livingOrb.setState('idle');
        updateStatusBadge('idle');
      }
    }
  }
}

// Click orb to speak or stop
talkButton.addEventListener('click', async () => {
  if (!isRunning) {
    runInteraction();
  } else {
    try {
      await invoke('stop_action');
    } catch (err) {
      console.error('Failed to cancel interaction on orb click:', err);
    }
  }
});

// Tauri Event Subscriptions
listen('presence_state_changed', (event) => {
  const state = event.payload;
  if (livingOrb) livingOrb.setState(state);
  updateStatusBadge(state);
});

listen('audio_level', (event) => {
  const level = typeof event.payload === 'number' ? event.payload : 0.0;
  if (livingOrb) livingOrb.setAudioLevel(level);
});

listen('wakeup', () => {
  console.log('Byte wakeup signal received!');
  runInteraction();
});

// Keyboard controls: Escape to dismiss/cancel, Ctrl+B toggle
window.addEventListener('keydown', async (event) => {
  if (event.key === 'Escape') {
    event.preventDefault();
    try {
      await invoke('stop_action');
    } catch (err) {
      console.error('Escape stop error:', err);
    }
  } else if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'b') {
    event.preventDefault();
    if (!isRunning) {
      runInteraction();
    } else {
      await invoke('stop_action');
    }
  }
});

// Cursor ignore click-through outside orb bounds
talkButton.addEventListener('mouseenter', async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(false);
  } catch (_) {}
});

talkButton.addEventListener('mouseleave', async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(true);
  } catch (_) {}
});

window.addEventListener('DOMContentLoaded', async () => {
  try {
    const { getCurrentWindow } = window.__TAURI__.window;
    await getCurrentWindow().setIgnoreCursorEvents(true);
    await invoke('reposition_presence');
  } catch (_) {}
});

// Initial state setup
updateStatusBadge('idle');
if (livingOrb) livingOrb.setState('idle');
